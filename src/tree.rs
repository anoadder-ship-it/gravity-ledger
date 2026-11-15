//! Append-only Merkle-accumulator, **incrementeel** bijgehouden.
//!
//! Eerdere versie: `root()` en `proof()` herbouwden de hele boom (179 ms root,
//! 193 ms bewijsopbouw bij 100.000 bladeren). Deze versie cachet elk niveau,
//! dus:
//!   * `push`     O(log n) — alleen de rechterpad verandert
//!   * `root`     O(1)     — het hoogste niveau heeft precies één knoop
//!   * `proof`    O(log n) — siblings rechtstreeks uit de cache
//!
//! De pairing en vulling zijn **identiek** aan de vorige versie (paren (0,1),
//! (2,3), …; een los kind wordt met zichzelf gepaard). Daarmee controleren de
//! bestaande aanvallen-tests niet alleen veiligheid maar ook equivalentie: een
//! slimmere datastructuur moet dezelfde wortel opleveren.
//!
//! Beveiligingskeuzes (twee keer gecontroleerd, beide gaten zaten in v1):
//!   1. domeinscheiding tussen blad / knoop / leeg;
//!   2. index-binding per blad — zonder die binding bindt de root niet aan het
//!      aantal bladeren en bindt een bewijs niet aan zijn positie.

use std::collections::VecDeque;

use crate::hash::{dsh, Digest};

const LEAF_TAG: &[u8] = b"GL/leaf/v1";
const NODE_TAG: &[u8] = b"GL/node/v0";
const EMPTY_TAG: &[u8] = b"GL/empty/v0";

/// Grootste diepte van een bewijs: één richting per bit van een u64-index.
/// Vast, niet `usize::BITS`, zodat elk platform hetzelfde aanvaardt.
pub const MAX_DIEPTE: usize = 64;

/// Het blad op positie `index`. De index gaat altijd als u64 little-endian
/// (8 byte) de hash in, op elk platform: de wortel hangt niet af van de
/// breedte van `usize`.
#[inline]
fn leaf_node(index: u64, leaf: &Digest) -> Digest {
    dsh(LEAF_TAG, &[&index.to_le_bytes(), leaf.as_slice()])
}

#[inline]
fn internal(left: &Digest, right: &Digest) -> Digest {
    dsh(NODE_TAG, &[left.as_slice(), right.as_slice()])
}

/// Bewijs dat een commitment op een bepaalde positie in de accumulator zit.
///
/// Alleen de siblings, van onderaf. De richting op niveau k volgt uit bit k
/// van de index (0: de knoop staat links, de sibling rechts). Er is dus niets
/// vrij te kiezen: bij gegeven index, blad en wortel is er hooguit één
/// codering die slaagt (stap 6). Het losse kind aan de rechterrand van een
/// oneven niveau staat links met zichzelf als sibling; dat volgt uit de index
/// vanzelf, want zijn positie is even.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Proof {
    /// Siblings van onderaf naar boven.
    pub path: Vec<Digest>,
}

impl Proof {
    /// Verifieert het bewijs voor `leaf` op `index`. De index is verplicht:
    /// zonder die positie is het bewijs niet aan een plek in de set gebonden,
    /// en hij bepaalt de richting op elk niveau.
    pub fn verify(&self, index: usize, leaf: &Digest, root: &Digest) -> bool {
        // Gerekend als u64 (verliesvrij vanuit usize), op elk platform gelijk.
        // Een u64-index heeft MAX_DIEPTE richtingen; een langer pad heeft geen
        // betekenis (en zou de schuif hieronder laten overlopen).
        let index = index as u64;
        if self.path.len() > MAX_DIEPTE { return false; }
        let mut acc = leaf_node(index, leaf);
        for (k, sib) in self.path.iter().enumerate() {
            acc = if (index >> k) & 1 == 0 { internal(&acc, sib) } else { internal(sib, &acc) };
        }
        &acc == root
    }

    pub fn depth(&self) -> usize { self.path.len() }

    /// Grootte van het bewijs: één digest per niveau.
    pub fn size_bytes(&self) -> usize {
        self.path.len() * crate::hash::DIGEST_BYTES
    }

    /// Draadformaat: de siblings achter elkaar, 64 byte per niveau, van onderaf.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.path.concat()
    }

    /// Leest het draadformaat. `None` bij een lengte die geen veelvoud van 64 is.
    pub fn from_bytes(b: &[u8]) -> Option<Proof> {
        if b.len() % crate::hash::DIGEST_BYTES != 0 { return None; }
        let path = b
            .chunks_exact(crate::hash::DIGEST_BYTES)
            .map(|c| c.try_into().expect("chunk van 64 byte"))
            .collect();
        Some(Proof { path })
    }
}

/// Eén update van de rechterpad, genummerd door de wortel die hij opleverde.
///
/// Dit is het stuk dat concurrentie mogelijk maakt. Een bewijs tegen wortel R
/// blijft aanvaard nadat anderen hebben toegevoegd, zolang R in de log staat
/// (`accepts_root`). Het bewijs wordt niet bijgewerkt; de log dient alleen om
/// R te herkennen.
#[derive(Clone, Debug)]
pub struct Change {
    /// Wortel direct na deze update; een bewijs kan tegen deze wortel gebouwd
    /// zijn. De wortel wordt niet getekend.
    pub root: Digest,
    /// Positie van het toegevoegde blad.
    pub index: usize,
}

/// Standaard aantal oude wortels waartegen een bewijs nog aanvaard wordt.
pub const DEFAULT_CHANGELOG: usize = 64;

#[derive(Clone, Debug, Default)]
pub struct Accumulator {
    /// Van buiten de crate niet te wijzigen: `push` gebruikt de lengte als
    /// index van het volgende blad.
    pub(crate) leaves: Vec<Digest>,
    /// `levels[k]` bevat alle knooppunten op niveau k (niveau 0 = bladeren).
    /// Niveau k heeft `ceil(n / 2^k)` knopen, dus het hoogste niveau heeft er
    /// precies één: de wortel.
    levels: Vec<Vec<Digest>>,
    /// Recente updates, oudste vooraan. Beperkt: een bewijs op een te oude
    /// wortel wordt geweigerd in plaats van geraden.
    changelog: VecDeque<Change>,
    max_log: usize,
}

impl Accumulator {
    pub fn new() -> Self {
        Self { leaves: Vec::new(), levels: Vec::new(), changelog: VecDeque::new(), max_log: DEFAULT_CHANGELOG }
    }

    /// Hoeveel wortels terug een bewijs nog aanvaard wordt.
    pub fn set_changelog_size(&mut self, n: usize) {
        self.max_log = n.max(1);
        while self.changelog.len() > self.max_log { self.changelog.pop_front(); }
    }

    /// Voegt een blad toe. Kost O(log n): alleen de pad vanaf het nieuwe
    /// blad naar boven verandert. Alleen binnen de crate: de ledger roept dit
    /// uitsluitend aan met `blad(niveau, commitment)` (stap 6).
    pub(crate) fn push(&mut self, commitment: Digest) -> usize {
        let index = self.leaves.len();
        self.leaves.push(commitment);

        if self.levels.is_empty() { self.levels.push(Vec::new()); }
        let leaf = leaf_node(index as u64, &commitment);
        self.levels[0].push(leaf);

        let mut k = 0;
        loop {
            let n = self.levels[k].len();
            // Een niveau met één knoop is al de wortel: daarboven komt niets.
            // (Bij n=1 is het blad zelf de wortel — dit miste in de eerste
            //  versie en gaf een extra knoop, dus een andere wortel.)
            if n < 2 { break; }
            let want = (n + 1) / 2;
            if self.levels.len() == k + 1 { self.levels.push(Vec::new()); }

            // Het ouderknooppunt van het laatste paar op dit niveau. Bij een
            // oneven aantal is dat het losse kind met zichzelf.
            let (left, right) = if n % 2 == 1 {
                (self.levels[k][n - 1], self.levels[k][n - 1])
            } else {
                (self.levels[k][n - 2], self.levels[k][n - 1])
            };
            let parent = internal(&left, &right);

            // Voeg toe als het niveau nog groeit, vervang als het bovenste
            // knooppunt hier opnieuw berekend wordt. (Pariteit alleen is fout:
            // bij de tweede push ontstaat dit niveau én voltooit het paar.)
            let have = self.levels[k + 1].len();
            if have < want { self.levels[k + 1].push(parent); }
            else { self.levels[k + 1][want - 1] = parent; }


            // Het hoogste niveau heeft één knoop: dat is de wortel, klaar.
            if self.levels[k + 1].len() == 1 && want == 1 { break; }
            k += 1;
        }

        self.changelog.push_back(Change { root: self.root(), index });
        while self.changelog.len() > self.max_log { self.changelog.pop_front(); }
        index
    }

    pub fn len(&self) -> usize { self.leaves.len() }
    pub fn is_empty(&self) -> bool { self.leaves.is_empty() }

    /// O(1): de wortel is het enige knooppunt op het hoogste niveau.
    pub fn root(&self) -> Digest {
        match self.levels.last() {
            Some(top) if !top.is_empty() => top[top.len() - 1],
            _ => dsh(EMPTY_TAG, &[]),
        }
    }

    /// Acceptatievenster voor concurrentie: is `root` een wortel die deze boom
    /// recent werkelijk had (de huidige, of een van de laatste `max_log`)?
    ///
    /// Dit is de hele concurrency-oplossing, en hij is klein omdat de boom
    /// append-only is. Een blad dat in wortel R zat, zit over tien updates nog
    /// steeds in de boom: bladeren worden hier nooit verwijderd of herschreven.
    /// Wie dus een bewijs tegen een iets verouderde wortel aanlevert, bewijst
    /// nog steeds iets warings — de vraag is alleen of die wortel nog binnen de
    /// window valt waarbinnen we hem kunnen verantwoorden.
    ///
    /// Een ouder wortel wordt geweigerd: niet omdat het bewijs onwaar zou zijn,
    /// maar omdat we dan geen begrensd bewijs van die wortel naar nu kunnen
    /// leveren. Falen boven gokken.
    pub fn accepts_root(&self, root: &Digest) -> bool {
        if *root == self.root() { return true; }
        self.changelog.iter().any(|c| &c.root == root)
    }

    /// Volledige verificatie zoals de keten die doet: het bewijs moet folden
    /// naar het aangereikte wortel, én dat wortel moet binnen het venster
    /// vallen. Een willekeurig oud (maar waar) bewijs van vóór het venster
    /// faalt hier, net als een vervalsing.
    pub fn verify_at(&self, proof: &Proof, index: usize, leaf: &Digest, root: &Digest) -> bool {
        proof.verify(index, leaf, root) && self.accepts_root(root)
    }

    /// O(log n): siblings uit de cache, geen herbouw.
    pub fn proof(&self, index: usize) -> Option<Proof> {
        if index >= self.leaves.len() { return None; }
        let mut path = Vec::with_capacity(self.levels.len());
        let mut idx = index;
        let mut k = 0;
        while k < self.levels.len() - 1 {
            let n = self.levels[k].len();
            // Los kind aan de rechterkant van een oneven niveau: sibling is menzelf.
            let sib = if idx % 2 == 0 {
                if idx + 1 < n { idx + 1 } else { idx }
            } else {
                idx - 1
            };
            path.push(self.levels[k][sib]);
            idx /= 2;
            k += 1;
        }
        Some(Proof { path })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf(x: u8) -> Digest { dsh(b"test/leaf", &[&[x]]) }

    /// Referentie-implementie: de triviale, volledige herbouw uit v1. Deze
    /// boom moet er exact hetzelfde uitzien, anders is de cache fout.
    fn root_na_herbouw(xs: &[Digest]) -> Digest {
        if xs.is_empty() { return dsh(EMPTY_TAG, &[]); }
        let mut level: Vec<Digest> = xs.iter().enumerate().map(|(i,l)| leaf_node(i as u64,l)).collect();
        while level.len() > 1 {
            let mut next = Vec::new();
            let mut i = 0;
            while i < level.len() {
                let l = level[i];
                let r = if i + 1 < level.len() { level[i+1] } else { level[i] };
                next.push(internal(&l, &r));
                i += 2;
            }
            level = next;
        }
        level[0]
    }

    fn tree(xs: &[u8]) -> Accumulator {
        let mut a = Accumulator::new();
        for &x in xs { a.push(leaf(x)); }
        a
    }

    #[test]
    fn cache_geeft_zelfde_wortel_als_herbouw() {
        // De kern van deze herschrijving: slimmer maar niet anders.
        for n in 0..=40usize {
            let xs: Vec<Digest> = (0..n as u8).map(leaf).collect();
            let mut a = Accumulator::new();
            for x in &xs { a.push(*x); }
            assert_eq!(a.root(), root_na_herbouw(&xs), "verschil bij n={}", n);
        }
    }

    #[test]
    fn lege_accumulator_is_stabiel() {
        assert_eq!(Accumulator::new().root(), Accumulator::new().root());
    }

    #[test]
    fn root_verandert_bij_elke_push() {
        let mut a = Accumulator::new();
        let mut seen: Vec<Digest> = vec![a.root()];
        for i in 0..17u8 {
            a.push(leaf(i));
            let r = a.root();
            assert!(!seen.contains(&r), "root herhaalde zich");
            seen.push(r);
        }
    }

    // ---- aanvallen: concurrentie -------------------------------------------------

    /// De concurrency-eis: een bewijs dat tegen een oudere wortel werd
    /// opgebouwd, moet nog steeds aanvaard worden nadat anderen hebben
    /// toegevoegd. Dit geldt voor élke combinatie van n bestaande bladeren en
    /// m volgende, inclusief de boomgroei-overgangen (1→2, 3→4, 7→8).
    #[test]
    fn bewijs_uit_het_verleden_blijdt_geldig() {
        for n in 1..=9usize {
            let mut a = Accumulator::new();
            for i in 0..n { a.push(leaf(i as u8)); }
            let snapshot = a.clone();
            let root_oud = a.root();

            for m in 1..=9usize {
                for j in 0..m { a.push(leaf((100 + j) as u8)); }
                assert!(a.accepts_root(&root_oud), "oude wortel verworpen: n={} m={}", n, m);

                for i in 0..n {
                    let p = snapshot.proof(i).expect("bewijs uit snapshot");
                    assert!(a.verify_at(&p, i, &leaf(i as u8), &root_oud),
                            "bewijs uit verleden verworpen: n={} m={} i={}", n, m, i);
                }
            }
        }
    }

    /// Een bewijs is vast aan zijn wortel: hetzelfde bewijs tegen de nieuwe
    /// wortel hoort te falen. Zonder deze eigenschap zou de verificator
    /// willekeurige wortels accepteren en is het venster waardeloos.
    #[test]
    fn oud_bewijs_op_verkeerde_wortel_faalt() {
        let mut a = Accumulator::new();
        for i in 0..4 { a.push(leaf(i)); }
        let snapshot = a.clone();
        let root_oud = a.root();
        for j in 0..5 { a.push(leaf((200 + j) as u8)); }
        let root_nieuw = a.root();

        for i in 0..4 {
            let p = snapshot.proof(i).unwrap();
            // Eerst: het bewijs is op zichzelf geldig. Zonder deze toets zou de
            // faaltest hieronder ook slagen als het bewijs gewoon kapot was.
            assert!(a.verify_at(&p, i, &leaf(i as u8), &root_oud),
                    "bewijs faalt tegen zijn eigen wortel (i={})", i);
            assert!(!p.verify(i, &leaf(i as u8), &root_nieuw),
                    "oud bewijs werd geaccepteerd op nieuwe wortel (i={})", i);
        }
    }

    /// Buiten het venster valt alles weg: een wortel die uit de log is
    /// verdwenen wordt geweigerd, ook al was hij ooit echt.
    #[test]
    fn wortel_buiten_het_venster_wordt_geweigerd() {
        let mut a = Accumulator::new();
        a.set_changelog_size(3);
        for i in 0..2 { a.push(leaf(i)); }
        let root_oud = a.root();
        for j in 0..12 { a.push(leaf((50 + j) as u8)); }

        assert!(!a.accepts_root(&root_oud), "verouderde wortel toch geaccepteerd");
        let bewijs = {
            let mut b = Accumulator::new();
            for i in 0..2 { b.push(leaf(i)); }
            b.proof(0).unwrap()
        };
        assert!(!a.verify_at(&bewijs, 0, &leaf(0), &root_oud),
                "verificatie buiten venster geslaagd");
    }

    /// Within het venster geldt dezelfde vervalsingsresistentie als daarvoor.
    #[test]
    fn bewijs_binnen_venster_is_nog_steeds_vast() {
        let mut a = Accumulator::new();
        for i in 0..3 { a.push(leaf(i)); }
        let snapshot = a.clone();
        let root_oud = a.root();
        for j in 0..6 { a.push(leaf((300 + j) as u8)); }

        let p = snapshot.proof(1).unwrap();
        assert!(a.verify_at(&p, 1, &leaf(1), &root_oud));

        for k in 0..p.path.len() {
            let mut v = p.clone();
            v.path[k][0] ^= 0x01;
            assert!(!a.verify_at(&v, 1, &leaf(1), &root_oud), "bitverdraaid bewijs geaccepteerd");
        }
        assert!(!a.verify_at(&p, 1, &leaf(2), &root_oud), "verkeerde bladeren geaccepteerd");
        assert!(!a.verify_at(&p, 0, &leaf(1), &root_oud), "verkeerde index geaccepteerd");
    }

    #[test]
    fn bewijs_werkt_voor_alle_posities_tot_33_bladeren() {
        for n in 1..=33usize {
            let a = tree(&(0..n as u8).collect::<Vec<u8>>());
            let root = a.root();
            for i in 0..n {
                let p = a.proof(i).expect("bewijs moet bestaan");
                assert!(p.verify(i, &leaf(i as u8), &root), "n={} idx={}", n, i);
            }
        }
    }

    #[test]
    fn root_bindt_aantal_bladeren() {
        assert_ne!(tree(&[0,1,2]).root(), tree(&[0,1,2,2]).root());
    }

    #[test]
    fn bewijs_bindt_positie() {
        let a = tree(&[5, 9, 5]);
        let (p0, p2) = (a.proof(0).unwrap(), a.proof(2).unwrap());
        assert_ne!(p0, p2);
        assert!(!p0.verify(2, &leaf(5), &a.root()), "positie is niet gebonden");
    }

    #[test]
    fn vervalsing_faalt() {
        let a = tree(&[0,1,2,3,4,5,6,7,8]);
        let root = a.root();
        let p = a.proof(3).unwrap();
        assert!(!p.verify(3, &leaf(4), &root));
        assert!(!p.verify(3, &leaf(3), &leaf(0)));
        assert!(!p.verify(4, &leaf(3), &root));
    }

    #[test]
    fn elke_siblingbitverdraaiing_faalt() {
        let a = tree(&(0..12u8).collect::<Vec<u8>>());
        let root = a.root();
        for idx in 0..12usize {
            let p = a.proof(idx).unwrap();
            for n in 0..p.path.len() {
                for bit in 0..8 {
                    let mut q = p.clone();
                    q.path[n][bit / 8] ^= 1 << (bit % 8);
                    assert!(!q.verify(idx, &leaf(idx as u8), &root),
                            "bitverdraaiing doorgekomen: idx={} niveau={} bit={}", idx, n, bit);
                }
            }
        }
    }

    #[test]
    fn index_buiten_bereeg_geeft_geen_bewijs() {
        let a = tree(&[1,2,3]);
        assert!(a.proof(3).is_none());
        assert!(a.proof(999).is_none());
    }

    #[test]
    fn volgorde_doet_ertoe() {
        assert_ne!(tree(&[1,2]).root(), tree(&[2,1]).root());
    }

    #[test]
    fn bewijsgrootte_groeit_logaritmisch() {
        for (n, max_diepte) in [(1usize,1usize),(2,1),(3,2),(8,3),(1_000,10),(10_000,14)] {
            let mut a = Accumulator::new();
            for i in 0..n { a.push(leaf((i % 251) as u8)); }
            let p = a.proof(n / 2).unwrap();
            assert!(p.depth() <= max_diepte, "n={} diepte={} > {}", n, p.depth(), max_diepte);
        }
    }

    /// R5 (review 2026-11-10), verplaatst uit tests/veiligheid.rs in stap 6
    /// omdat `push` pub(crate) werd; bladen en asserts ongewijzigd. Elk bewijs
    /// klopt alleen voor zijn eigen positie en blad (n = 1..=70); ingekorte en
    /// verlengde paden falen; het venster aanvaardt precies de laatste 64
    /// wortels: na 200 opnames zijn dat wortels 136..=199.
    #[test]
    fn r5_boom_en_venster() {
        let leaf = |i: usize| dsh(b"t", &[&(i as u64).to_le_bytes()]);
        for n in 1..=70usize {
            let mut a = Accumulator::new();
            for i in 0..n { a.push(leaf(i)); }
            let root = a.root();
            for i in 0..n {
                let p = a.proof(i).unwrap();
                assert!(p.verify(i, &leaf(i), &root), "n={n} i={i}: echt bewijs faalt");
                for j in (0..n).filter(|&j| j != i) {
                    assert!(!p.verify(j, &leaf(i), &root), "n={n}: bewijs {i} geldt op positie {j}");
                    assert!(!p.verify(i, &leaf(j), &root), "n={n}: bewijs {i} geldt voor blad {j}");
                    assert!(!p.verify(j, &leaf(j), &root), "n={n}: bewijs {i} geldt voor ({j},{j})");
                }
                let mut q = p.clone();
                if q.path.pop().is_some() { assert!(!q.verify(i, &leaf(i), &root), "n={n} i={i}: ingekort pad"); }
                let mut q = p.clone();
                q.path.push(root);
                assert!(!q.verify(i, &leaf(i), &root), "n={n} i={i}: verlengd pad");
            }
        }
        let mut a = Accumulator::new();
        let mut roots = vec![];
        for i in 0..200 { a.push(leaf(i)); roots.push(a.root()); }
        let aanvaard: Vec<usize> = (0..200).filter(|&i| a.accepts_root(&roots[i])).collect();
        assert_eq!(aanvaard, (136..200).collect::<Vec<_>>(), "venster is niet precies wortels 136..=199");
        let bewijs_bij = |n: usize| { let mut b = Accumulator::new(); for i in 0..n { b.push(leaf(i)); } b.proof(5).unwrap() };
        assert!(a.verify_at(&bewijs_bij(137), 5, &leaf(5), &roots[136]), "waar bewijs binnen het venster geweigerd");
        assert!(!a.verify_at(&bewijs_bij(101), 5, &leaf(5), &roots[100]), "waar bewijs buiten het venster aanvaard");
    }

    /// Stap 7e: de index gaat als u64 little-endian, 8 byte, de hash in,
    /// met de hand uitgeschreven en los van `to_le_bytes`. Op 64-bit kan deze
    /// test niet rood zijn geweest (daar gaf `usize` al dezelfde 8 byte); hij
    /// borgt dat het blad, en via `push` de wortel, ook na een wijziging op
    /// elk platform dezelfde bytes houdt.
    #[test]
    fn blad_hasht_index_als_8_byte_little_endian() {
        let x = leaf(42);
        let i: u64 = 0x0102_0304_0506_0708;
        let handmatig = [0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01];
        assert_eq!(leaf_node(i, &x), dsh(LEAF_TAG, &[&handmatig, &x]));
        // Index 0 via push: de wortel van een boom met één blad is dat blad.
        let mut a = Accumulator::new();
        a.push(x);
        assert_eq!(a.root(), dsh(LEAF_TAG, &[&[0u8; 8], &x]));
        // Index 1 via push: knoop over (blad 0, blad 1).
        a.push(leaf(43));
        let b0 = dsh(LEAF_TAG, &[&[0u8; 8], &x]);
        let b1 = dsh(LEAF_TAG, &[&[1, 0, 0, 0, 0, 0, 0, 0], &leaf(43)]);
        assert_eq!(a.root(), internal(&b0, &b1));
    }
}
