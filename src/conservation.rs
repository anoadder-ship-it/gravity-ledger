//! Waardebehoud door **telling**, niet door sommatie.
//!
//! Dit bestand is de kern van het hele argument. RingCT afdwingen van
//! `Σ in = Σ out` vraagt een additief homomorfe commitment (Pedersen), en die
//! breekt als eerste onder een quantumcomputer — zowel hiding als binding, dus
//! niet alleen meelezen maar *values printen*.
//!
//! Met vaste denominaties hoeft er niets opgeteld te worden. Per niveau moet de
//! **stroom noten** gewoon kloppen, en splitsen/voegen zijn lokale, structurele
//! feiten in plaats van veldrekenen. Range-proof: nul bytes.
//!
//! Balans per niveau `d`:
//! ```text
//!   ins[d] + 2·splits[d+1] + merges[d]  =  outs[d] + fee[d] + splits[d] + 2·merges[d+1]
//!   └────────── aanbod ──────────┘         └──────────── vraag ────────────┘
//! ```

use crate::note::DENOM_LEVELS;

pub type Counts = [u32; DENOM_LEVELS];

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
    /// Protocolregel: klopt de notenstroom per niveau?
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

    /// **Alleen voor tests en voor de audit-laag.** De naïeve sommatie die we
    /// protocol-ontzeggen, als waarheidsorakel.
    pub fn total_value(counts: &Counts) -> u64 {
        counts
            .iter()
            .enumerate()
            .map(|(lvl, n)| (*n as u64) << lvl)
            .sum()
    }

    /// Aggregate public record voor één epoch (laag T): aantallen per niveau.
    /// Dit is wat een auditor ziet — genoeg voor de geldvoorraad, niet genoeg
    /// om ook maar één betaling te herleiden.
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
}
