//! Boekhouding per emmer (epoch, niveau): opnames en bestedingen.
//!
//! De besteding zelf staat sinds stap 7 in `besteding.rs` (zichtbaar en
//! getekend). De oude `spend` met een vertrouwde verifieerder, het oude
//! `Spend`-record en `DOELMENIGTE` zijn in stap 7 verwijderd; ze bestonden
//! tot en met stap 6 (die geschiedenis is niet gepubliceerd). Het menigtemodel uit
//! STATUS.md 3d blijft onderzoek.
//!
//! Wat hier overblijft, gebruikt de besteding als extra verdediging (controle
//! 8): het aantal nog besteedbare munten in een emmer mag nooit negatief
//! worden.

use std::collections::BTreeMap;

use crate::note::Note;
use crate::Ledger;

/// Hoe vol de emmers zijn: per (epoch, niveau) het aantal opgenomen noten.
pub type Buckets = BTreeMap<(u64, u8), usize>;

/// Hoe vaak uit een emmer besteed is. Samen met `Buckets` geeft dit het aantal
/// nog besteedbare noten.
pub type Spents = BTreeMap<(u64, u8), usize>;

/// Neemt een noot op én telt hem in zijn emmer: roept alleen `Ledger::mint`
/// aan, dat het werk doet (één implementatie). Blijft bestaan voor bestaande
/// aanroepers. Ook `Ledger::besteed` vult de emmers: de uitgang telt in
/// emmer (epoch van de ledger, niveau).
pub fn mint_to_ledger(l: &mut Ledger, note: &Note) -> usize {
    l.mint(note)
}

/// Aantal opnames in een emmer. Dit is de kandidatenlijst voor de vraag
/// "welke nieuwe commitment is van mij?" — elke opname komt ervoor in aanmerking.
pub fn menigte(buckets: &Buckets, epoch: u64, level: u8) -> usize {
    buckets.get(&(epoch, level)).copied().unwrap_or(0)
}

/// Aantal nog besteedbare noten in een emmer. Meer bestedingen dan opnames is
/// een kapotte staat en wordt gemeld, niet op nul afgerond.
pub fn live(buckets: &Buckets, spent: &Spents, epoch: u64, level: u8) -> Result<usize, SpendFout> {
    let opnames = menigte(buckets, epoch, level);
    let bestedingen = spent.get(&(epoch, level)).copied().unwrap_or(0);
    opnames
        .checked_sub(bestedingen)
        .ok_or(SpendFout::NegatieveEmmer { epoch, level, opnames, bestedingen })
}

/// Kandidaten die een waarnemer overhoudt: alle posities in de accumulator die
/// tot dezelfde emmer behoren (menigtemodel, onderzoek).
pub fn kandidaten(l: &Ledger, epoch: u64, level: u8) -> Vec<usize> {
    (0..l.acc.leaves.len())
        .filter(|&i| l.bucket_of(i) == Some((epoch, level)))
        .collect()
}

#[derive(Debug, PartialEq, Eq)]
pub enum SpendFout {
    /// Meer bestedingen dan opnames: de tellers zijn beschadigd.
    NegatieveEmmer { epoch: u64, level: u8, opnames: usize, bestedingen: usize },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::note::{ChainId, Secret};

    fn chain() -> ChainId {
        let mut id = [0u8; 32];
        id[..4].copy_from_slice(b"test");
        ChainId(id)
    }
    fn noot(level: u8, epoch: u64, ki: u8) -> Note {
        let mut s = [0u8; 32];
        s[0] = level; s[1] = epoch as u8; s[2] = ki;
        Note { level, epoch, owner: [0u8; 64], secret: Secret(s) }
    }

    /// De menigte telt per (epoch, niveau) en de kandidatenlijst is precies die
    /// emmer: dat ís de anonimiteitset.
    #[test]
    fn menigte_telt_per_emmer() {
        let mut l = Ledger::new(chain());
        for i in 0..12 { mint_to_ledger(&mut l, &noot(3, 4, i)); }
        for i in 0..5 { mint_to_ledger(&mut l, &noot(9, 4, i)); }
        for i in 0..7 { mint_to_ledger(&mut l, &noot(3, 5, i)); }

        assert_eq!(menigte(&l.buckets, 4, 3), 12);
        assert_eq!(menigte(&l.buckets, 4, 9), 5);
        assert_eq!(menigte(&l.buckets, 5, 3), 7);
        assert_eq!(menigte(&l.buckets, 9, 9), 0);

        let k = kandidaten(&l, 4, 3);
        assert_eq!(k.len(), 12, "kandidatenlijst is niet de emmer");
    }

    /// Koppelbaarheid is een eigenschap van de emmergrootte: in een emmer van
    /// één is de winnaar aanwijsbaar, in een volle emmer niet. (Voorheen met
    /// `DOELMENIGTE`; dezelfde waarde, 100, staat nu hier.)
    #[test]
    fn dunne_emmer_verraadt_volledig() {
        const VOL: usize = 100;
        let mut een = Ledger::new(chain());
        mint_to_ledger(&mut een, &noot(2, 1, 0));
        assert_eq!(kandidaten(&een, 1, 2).len(), 1);

        let mut vol = Ledger::new(chain());
        for i in 0..VOL { mint_to_ledger(&mut vol, &noot(2, 1, i as u8)); }
        assert_eq!(kandidaten(&vol, 1, 2).len(), VOL);
    }
}
