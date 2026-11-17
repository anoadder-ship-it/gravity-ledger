//! gravity-ledger — onderzoeksprototype, niet geauditeerd.
//!
//! Munten met vaste denominaties 2^0..2^32 in een Merkle-accumulator op BLAKE3,
//! en een zichtbare, met Ed25519 getekende besteding (`Ledger::besteed`).
//! Anonimiteit is niet gebouwd (onderzoek, STATUS.md sectie 3d). Zie
//! `README.md` en `bench.rs`.

pub mod bench;
pub mod besteding;
pub mod conservation;
pub mod handtekening;
pub mod hash;
#[cfg(test)]
#[rustfmt::skip]
pub mod kat;
pub mod note;
pub mod spend;
pub mod tree;

pub use conservation::Shape;
pub use hash::Digest;
pub use note::{ChainId, Note, Secret};
pub use tree::Accumulator;

/// Een mini-ledger: de accumulator, de verbrande nullifiers en de tellers per
/// emmer. Deze structuur alleen maakt nog geen veilig protocol.
///
/// Een blad komt alleen via `mint` of `besteed` (de uitgang) in de boom, dus
/// altijd als `blad(niveau, commitment)`. Van buiten de crate is de toestand alleen te
/// lezen (stap 6). Toegestaan:
///
/// ```
/// use gravity_ledger::{ChainId, Ledger, Note, Secret};
/// let mut l = Ledger::new(ChainId([0; 32]));
/// let n = Note { level: 3, epoch: 0, owner: [0; 64], secret: Secret([1; 32]) };
/// let i = l.mint(&n);
/// assert_eq!(l.acc().len(), 1);
/// assert!(l.acc().proof(i).is_some());
/// ```
///
/// Een kale commitment rechtstreeks in de accumulator van de ledger zetten,
/// compileert niet:
///
/// ```compile_fail,E0616
/// use gravity_ledger::{ChainId, Ledger};
/// let mut l = Ledger::new(ChainId([0; 32]));
/// l.acc.push([7u8; 64]);
/// ```
///
/// Ook niet in een losse accumulator:
///
/// ```compile_fail,E0624
/// let mut a = gravity_ledger::Accumulator::new();
/// a.push([7u8; 64]);
/// ```
///
/// En de nullifierverzameling of de tellers zijn van buiten niet te wijzigen:
///
/// ```compile_fail,E0616
/// use gravity_ledger::{ChainId, Ledger};
/// let mut l = Ledger::new(ChainId([0; 32]));
/// l.nullified.clear();
/// ```
#[derive(Debug)]
pub struct Ledger {
    pub(crate) chain: ChainId,
    pub(crate) epoch: u64,
    pub(crate) acc: Accumulator,
    pub(crate) nullified: std::collections::BTreeSet<Digest>,
    /// Hoe vol elke emmer (epoch, niveau) is. `besteed` gebruikt dit in
    /// controle 8. De besteding is zichtbaar; alleen in het menigtemodel
    /// (onderzoek, STATUS.md 3d) zou dit de anonimiteit bepalen.
    pub(crate) buckets: spend::Buckets,
    /// Emmer per accumulatorpositie, in volgorde van opname.
    pub(crate) minted: Vec<(u64, u8)>,
    /// Aantal bestedingen per emmer; saam met `buckets` het aantal nog
    /// besteedbare noten.
    pub(crate) spent: spend::Spents,
}

impl Ledger {
    pub fn new(chain: ChainId) -> Self {
        Self {
            chain,
            epoch: 0,
            acc: Accumulator::new(),
            nullified: std::collections::BTreeSet::new(),
            buckets: spend::Buckets::new(),
            minted: Vec::new(),
            spent: spend::Spents::new(),
        }
    }

    /// Neemt een noot op in de blinde accumulator (storten) en telt hem in
    /// zijn emmer (epoch, niveau van de noot). Dit is de enige implementatie
    /// van munten; `spend::mint_to_ledger` roept alleen dit aan.
    ///
    /// Stap 9 van `besteed` doet voor de uitgang dezelfde telling (één blad in
    /// de boom, één regel in `minted`, één erbij in `buckets`), maar deelt de
    /// uitgang in op het epoch van de ledger; `mint` deelt de munt in op haar
    /// eigen epoch (`note.epoch`). Zo lopen boom, `minted` en `buckets` nooit
    /// uiteen, en kan een opgenomen noot niet de plek van een andere in zijn
    /// emmer innemen.
    ///
    /// Publiek omdat de vraag wie munten mag uitgeven nog open staat
    /// (STATUS.md sectie 6, punt 6); `mint` controleert niet wie opneemt, en
    /// ook niet het niveau of het epoch van de noot.
    pub fn mint(&mut self, note: &Note) -> usize {
        let idx = self
            .acc
            .push(blad(note.level, &note.commitment(&self.chain)));
        let emmer = (note.epoch, note.level);
        self.minted.push(emmer);
        *self.buckets.entry(emmer).or_insert(0) += 1;
        idx
    }

    pub fn root(&self) -> Digest {
        self.acc.root()
    }

    /// Huidig epoch van de ledger.
    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    /// De keten waaraan deze ledger gebonden is.
    pub fn chain(&self) -> &ChainId {
        &self.chain
    }

    /// De accumulator, alleen lezen. Toevoegen gaat uitsluitend via `mint` en
    /// `besteed`, dus altijd als `blad(niveau, commitment)`.
    pub fn acc(&self) -> &Accumulator {
        &self.acc
    }

    /// Opnames per emmer, alleen lezen.
    pub fn buckets(&self) -> &spend::Buckets {
        &self.buckets
    }

    /// Bestedingen per emmer, alleen lezen.
    pub fn spent(&self) -> &spend::Spents {
        &self.spent
    }

    /// Is deze nullifier al verbrand? Verbranden gebeurt alleen in
    /// `besteed` (stap 7).
    pub fn is_verbrand(&self, nullifier: &Digest) -> bool {
        self.nullified.contains(nullifier)
    }

    /// Publieke lidmaatschapscontrole: zit `commitment` met niveau `level` op
    /// positie `index` in een wortel uit het venster? Het niveau is invoer van
    /// de verifieerder, niet iets dat uit de opening volgt.
    pub fn is_lid(
        &self,
        proof: &tree::Proof,
        index: usize,
        level: u8,
        commitment: &Digest,
        root: &Digest,
    ) -> bool {
        self.acc
            .verify_at(proof, index, &blad(level, commitment), root)
    }

    /// Emmer van een positie in de accumulator, voor de kandidatenlijst.
    /// `None` als de positie buiten bereik ligt.
    pub fn bucket_of(&self, index: usize) -> Option<(u64, u8)> {
        self.minted.get(index).copied()
    }
}

/// Het blad in de accumulator: `H("GL/blad/v1", niveau, commitment)`.
///
/// Het niveau staat hier publiek naast de commitment (stap 3, besluit sectie
/// 0). Zo kan de keten het niveau van een opgenomen noot controleren met
/// alleen publieke gegevens, ook voor een uitgang waarvan ze de opening nooit
/// ziet. De commitment zelf bindt het niveau ook; wie een blad maakt met een
/// ander niveau dan in de commitment, maakt een noot die niet te besteden is.
pub fn blad(level: u8, commitment: &Digest) -> Digest {
    hash::dsh(b"GL/blad/v1", &[&[level], commitment.as_slice()])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chain() -> ChainId {
        ChainId([9u8; 32])
    }

    fn note(level: u8, seed: u8) -> Note {
        let mut s = [0u8; 32];
        s[..8].copy_from_slice(&[
            level,
            seed,
            level ^ seed,
            seed,
            level,
            seed,
            level ^ seed,
            seed,
        ]);
        s[8..16].copy_from_slice(&[seed; 8]);
        for i in 0..24 {
            s[16 + (i % 16)] ^= level
                .wrapping_mul(31)
                .wrapping_add(seed)
                .wrapping_add(i as u8);
        }
        Note {
            level,
            epoch: 0,
            owner: [0u8; 64],
            secret: Secret(s),
        }
    }

    #[test]
    fn verschillende_noten_hebben_verschillende_images() {
        let c = chain();
        assert_ne!(note(4, 1).nullifier(&c), note(4, 2).nullifier(&c));
    }

    #[test]
    fn image_is_ketengebonden() {
        let n = note(4, 1);
        assert_ne!(n.nullifier(&chain()), n.nullifier(&ChainId([8u8; 32])));
    }

    #[test]
    fn accumulator_groeit_met_mint() {
        let mut l = Ledger::new(chain());
        let before = l.root();
        l.mint(&note(2, 7));
        assert_ne!(l.root(), before);
        assert_eq!(l.acc.len(), 1);
    }
}

#[cfg(test)]
mod tests_blok1 {
    use super::*;

    #[test]
    fn blok1_standaard_zes_testen() {
        let mut l = Ledger::new(ChainId([9u8; 32]));
        l.mint(&Note {
            level: 4,
            epoch: 0,
            owner: [0u8; 64],
            secret: Secret([7u8; 32]),
        });
        assert_eq!(l.acc.len(), 1);
    }
}
