//! Waardebehoud door **telling**, niet door sommatie.
//!
//! `Shape` beschrijft een overdracht als aantallen noten per niveau;
//! `Shape::check` controleert of die aantallen per niveau kloppen. RingCT
//! dwingt `Σ in = Σ out` af met een additief homomorfe commitment (Pedersen).
//! Met vaste denominaties hoeft er niets opgeteld te worden: per niveau moet
//! de **stroom noten** kloppen, en splitsen en voegen zijn tellingen per
//! niveau in plaats van veldrekenen. Range-proof: nul bytes.
//!
//! Balans per niveau `d`:
//! ```text
//!   ins[d] + 2·splits[d+1] + merges[d]  =  outs[d] + fee[d] + splits[d] + 2·merges[d+1]
//!   └────────── aanbod ──────────┘         └──────────── vraag ────────────┘
//! ```
//!
//! **`Ledger::besteed` roept `Shape::check` niet aan.** Een besteding heeft
//! precies één invoer en één uitgang, en `besteed` gebruikt het niveau uit het
//! record voor allebei de bladen: `blad(niveau, C_in)` bij het lidmaatschap
//! (controle 5) en `blad(niveau, C_uit)` bij het opnemen van de uitgang
//! (stap 9). De vorm van een besteding is dus altijd `ins[n] = outs[n] = 1`
//! en verder nul, en voor die vorm kan `check` niets weigeren (test
//! `een_invoer_een_uitgang`, alle 33 niveaus). `check` wordt
//! pas nodig bij bestedingen met meer invoeren of uitgangen, of met splitsen,
//! voegen of fooi; die bestaan nu niet.
//!
//! Buiten dit bestand gebruiken alleen `r4_behoud_omgekeerd`
//! (`tests/veiligheid.rs`) en de meting `meet_ladder_en_bewijzen`
//! (`src/bench.rs`) `Shape`.

use crate::note::DENOM_LEVELS;

pub type Counts = [u32; DENOM_LEVELS];

/// Een telling met nul op elk niveau. Alleen binnen dit bestand aangeroepen
/// (door `Shape::default` en in de tests hieronder); daarbuiten nergens.
pub fn zero() -> Counts {
    [0u32; DENOM_LEVELS]
}

/// De vorm van één overdracht: aantallen per niveau, geen bedragen.
#[derive(Clone, Copy, Debug)]
pub struct Shape {
    pub ins: Counts,
    pub outs: Counts,
    pub fee: Counts,
    /// `splits[d]`: noten van niveau d die worden gesplitst → 2 noten van d−1.
    pub splits: Counts,
    /// `merges[d]`: noten van niveau d die ontstaan uit 2 noten van d−1.
    pub merges: Counts,
}

impl Default for Shape {
    fn default() -> Self {
        Self {
            ins: zero(),
            outs: zero(),
            fee: zero(),
            splits: zero(),
            merges: zero(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Violation {
    /// Niveau 0 is ondeelbaar — er bestaat geen halve eenheid.
    IndivisibleSplit,
    /// Er is geen niveau onder 0 om naartoe te voegen.
    MergeAtFloor,
    /// Stroom klopt niet op één niveau: hier zou waarde ontstaan of verdwijnen.
    Imbalance { level: u8, aanbod: u64, vraag: u64 },
    /// Een som past niet in u64. Met de huidige grenzen kan dat niet: elke
    /// telling is een u32, dus ten hoogste 2^32 − 1. Het aanbod is hooguit
    /// 4 · (2^32 − 1) = 17.179.869.180 en de vraag hooguit 5 · (2^32 − 1) =
    /// 21.474.836.475, ver onder 2^64 − 1. De controles blijven als vangnet
    /// voor het geval de grenzen veranderen.
    Overflow,
}

#[inline]
fn g(a: &Counts, d: usize) -> u64 {
    if d < DENOM_LEVELS {
        a[d] as u64
    } else {
        0
    }
}

impl Shape {
    /// Klopt de notenstroom per niveau? Weigert, in deze volgorde:
    /// - `splits[0] != 0`: `IndivisibleSplit`;
    /// - `merges[0] != 0`: `MergeAtFloor`;
    /// - daarna per niveau d van 0 tot en met 32: `Overflow` als aanbod of
    ///   vraag niet in u64 past (kan niet, zie `Violation::Overflow`), en
    ///   `Imbalance` als `ins[d] + 2·splits[d+1] + merges[d]` verschilt van
    ///   `outs[d] + fee[d] + splits[d] + 2·merges[d+1]`. Boven niveau 32 telt
    ///   als nul. De eerste weigering wordt teruggegeven.
    ///
    /// `Ledger::besteed` roept dit niet aan: voor de enige vorm die een
    /// besteding heeft (één invoer en één uitgang op hetzelfde niveau) kan
    /// `check` niets weigeren. Zie de moduledocumentatie.
    pub fn check(&self) -> Result<(), Violation> {
        if self.splits[0] != 0 {
            return Err(Violation::IndivisibleSplit);
        }
        if self.merges[0] != 0 {
            return Err(Violation::MergeAtFloor);
        }

        for d in 0..DENOM_LEVELS {
            let split_below =
                2u64.checked_mul(g(&self.splits, d + 1)).ok_or(Violation::Overflow)?;
            let aanbod = g(&self.ins, d)
                .checked_add(split_below)
                .and_then(|v| v.checked_add(g(&self.merges, d)))
                .ok_or(Violation::Overflow)?;

            let merge_above =
                2u64.checked_mul(g(&self.merges, d + 1)).ok_or(Violation::Overflow)?;
            let vraag = g(&self.outs, d)
                .checked_add(g(&self.fee, d))
                .and_then(|v| v.checked_add(g(&self.splits, d)))
                .and_then(|v| v.checked_add(merge_above))
                .ok_or(Violation::Overflow)?;

            if aanbod != vraag {
                return Err(Violation::Imbalance { level: d as u8, aanbod, vraag });
            }
        }
        Ok(())
    }

    /// De sommatie `Σ aantal · 2^niveau`, als waarheidsorakel in tests
    /// (`telling_is_equivalent_aan_sommatie`, `een_split_is_waarde_neutraal`,
    /// `r4_behoud_omgekeerd`); elders wordt hij niet aangeroepen. Er is geen
    /// audit-laag. De som kan in een debug-build overlopen (STATUS.md
    /// sectie 6, punt 13b).
    pub fn total_value(counts: &Counts) -> u64 {
        counts
            .iter()
            .enumerate()
            .map(|(lvl, n)| (*n as u64) << lvl)
            .sum()
    }

    /// Geeft `(ins, outs, fee)` terug. Wordt nu nergens aangeroepen, ook niet
    /// in tests; er is geen epochrecord en geen auditor in de code.
    pub fn aggregate(&self) -> (Counts, Counts, Counts) {
        (self.ins, self.outs, self.fee)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self, m: u64) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (self.0 >> 33) % m
        }
    }

    /// Bouw een geldige transactie door *operaties* in plaats van door getallen:
    /// splitsen en voegen behoudt waarde per constructie.
    fn simuleer(rng: &mut Lcg, rnd: usize) -> Shape {
        let mut s = Shape::default();
        let mut pool: Counts = zero();

        for _ in 0..rnd {
            let lvl = rng.next(DENOM_LEVELS as u64) as usize;
            s.ins[lvl] += 1;
            pool[lvl] += 1;
        }

        for _ in 0..24 {
            let lvl = rng.next(DENOM_LEVELS as u64) as usize;
            if pool[lvl] == 0 {
                continue;
            }
            match rng.next(3) {
                0 if lvl > 0 => {
                    pool[lvl] -= 1;
                    pool[lvl - 1] += 2;
                    s.splits[lvl] += 1;
                }
                1 if lvl + 1 < DENOM_LEVELS && pool[lvl] >= 2 => {
                    pool[lvl] -= 2;
                    pool[lvl + 1] += 1;
                    s.merges[lvl + 1] += 1;
                }
                _ => {}
            }
        }

        // Fooi: alles op niveau 0 mag op, de rest wordt output.
        for lvl in 0..DENOM_LEVELS {
            let n = pool[lvl];
            if n == 0 {
                continue;
            }
            if lvl == 0 {
                let f = rng.next((n + 1) as u64) as u32;
                s.fee[0] += f;
                s.outs[0] += n - f;
            } else {
                s.outs[lvl] += n;
            }
        }
        s
    }

    #[test]
    fn telling_is_equivalent_aan_sommatie() {
        // Het kernbewering: wat het protocol afdwingt (telling) moet exact
        // samenvallen met wat waarheid is (sommatie).
        let mut rng = Lcg(0x5EED);
        for r in 1..400 {
            let s = simuleer(&mut rng, r % 12 + 1);
            assert_eq!(s.check(), Ok(()), "valse negatief bij ronde {}", r);
            let i = Shape::total_value(&s.ins);
            let o = Shape::total_value(&s.outs) + Shape::total_value(&s.fee);
            assert_eq!(i, o, "telling/sommatie divergeerden bij ronde {}", r);
        }
    }

    #[test]
    /// Eén extra noot van welk niveau dan ook moet afwijzen. Dit is de
    /// "waarde printen"-aanval, en die is hier puur tellend geblokkeerd.
    fn extra_noot_op_elk_niveau_wordt_ontdekt() {
        let mut rng = Lcg(1234);
        for lvl in 0..DENOM_LEVELS {
            let mut s = simuleer(&mut rng, 6);
            s.outs[lvl] += 1;
            assert!(
                matches!(s.check(), Err(Violation::Imbalance { .. })),
                "niveau {} liet valse waarde toe",
                lvl
            );
        }
    }

    #[test]
    fn oneindig_splitsen_wordt_verboden() {
        let mut s = Shape::default();
        s.ins[0] = 1;
        s.splits[0] = 1; // niveau 0 is ondeelbaar
        s.outs[0] = 1;
        assert_eq!(s.check(), Err(Violation::IndivisibleSplit));
    }

    #[test]
    fn voegen_onder_niveau_0_wordt_verboden() {
        let mut s = Shape::default();
        s.merges[0] = 1;
        assert_eq!(s.check(), Err(Violation::MergeAtFloor));
    }

    #[test]
    fn een_split_is_waarde_neutraal() {
        // 1× niveau 3 → 2× niveau 2
        let mut s = Shape::default();
        s.ins[3] = 1;
        s.splits[3] = 1;
        s.outs[2] = 2;
        assert_eq!(s.check(), Ok(()));
        assert_eq!(Shape::total_value(&s.ins), Shape::total_value(&s.outs));
    }

    #[test]
    fn een_merge_is_waarde_neutraal() {
        // 2× niveau 2 → 1× niveau 3
        let mut s = Shape::default();
        s.ins[2] = 2;
        s.merges[3] = 1;
        s.outs[3] = 1;
        assert_eq!(s.check(), Ok(()));
    }

    #[test]
    fn lege_transactie_is_geldig() {
        assert_eq!(Shape::default().check(), Ok(()));
    }

    /// De vorm van `Ledger::besteed`: één invoer en één uitgang, geen fooi,
    /// splitsen of voegen. Op hetzelfde niveau wordt hij voor alle 33 niveaus
    /// aanvaard; voor elk van de 33 · 32 = 1.056 paren verschillende niveaus
    /// geweigerd, met de onbalans op het laagste van de twee niveaus.
    #[test]
    fn een_invoer_een_uitgang() {
        let vorm = |a: usize, b: usize| {
            let mut s = Shape::default();
            s.ins[a] = 1;
            s.outs[b] = 1;
            s
        };
        for n in 0..DENOM_LEVELS {
            assert_eq!(vorm(n, n).check(), Ok(()), "niveau {n} geweigerd");
        }
        let mut geweigerd = 0;
        for a in 0..DENOM_LEVELS {
            for b in (0..DENOM_LEVELS).filter(|&b| b != a) {
                let (aanbod, vraag) = if a < b { (1, 0) } else { (0, 1) };
                let level = a.min(b) as u8;
                assert_eq!(vorm(a, b).check(), Err(Violation::Imbalance { level, aanbod, vraag }),
                           "invoer {a}, uitgang {b}");
                geweigerd += 1;
            }
        }
        assert_eq!(geweigerd, 1_056);
    }
}
