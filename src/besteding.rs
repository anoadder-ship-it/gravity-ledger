//! Stap 7: de zichtbare, getekende besteding (besluit sectie 0, punt 2 en 3).
//!
//! De besteder toont de opening van één munt, een lidmaatschapsbewijs tegen
//! een wortel in het venster, de commitment van één uitgang van hetzelfde
//! niveau en een Ed25519-handtekening. De keten rekent zelf de commitment, het
//! blad en de nullifier uit; niets daarvan wordt van de besteder aangenomen.
//! Er is geen anonimiteit: invoer en uitgang zijn aan elkaar te koppelen.
//!
//! **Record v1** (vaste breedtes, getallen u64 little-endian), totaal
//! `276 + 64 × diepte` byte:
//!
//! | veld | byte |
//! |---|---|
//! | versie (`0x01`) | 1 |
//! | schema (`0x01` = Ed25519) | 1 |
//! | index_in | 8 |
//! | niveau (< 33) | 1 |
//! | epoch_in | 8 |
//! | geheim_in | 32 |
//! | pk | 32 |
//! | wortel | 64 |
//! | diepte (≤ 64) | 1 |
//! | bewijs | 64 × diepte |
//! | C_uit | 64 |
//! | handtekening | 64 |
//!
//! **Controles**, in deze volgorde; de toestand verandert pas in stap 9:
//! 1. formaat, versie, schema, niveau, diepte;
//! 2. pk inlezen via de trait;
//! 3. `owner = owner_hash(schema, pk)`, `C_in = commitment(chain, niveau,
//!    epoch_in, owner, geheim_in)`;
//! 4. de wortel valt in het venster;
//! 5. lidmaatschap van `blad(niveau, C_in)` op `index_in`, door de keten zelf
//!    uitgerekend;
//! 6. `N = H("GL/nullify/v1", chain, index_in, C_in)` bestaat nog niet;
//! 7. handtekening (`verify_strict`) over
//!    `M = dsh("GL/besteding/v1", [chain, versie, schema, N, niveau, C_uit])`;
//! 8. de emmer volgt uit `minted[index_in]`; er moet iets besteedbaar zijn;
//! 9. in één keer: N toevoegen, `spent` van die emmer + 1, `blad(niveau,
//!    C_uit)` opnemen in emmer (epoch van de ledger, niveau).

use crate::handtekening::{Ed25519, Handtekening};
use crate::hash::{dsh, Digest, DIGEST_BYTES};
use crate::note::{owner_hash, ChainId, Note, Secret, DENOM_LEVELS};
use crate::tree::Proof;
use crate::{blad, spend, Ledger};

/// Versie van het recordformaat.
pub const VERSIE: u8 = 1;
/// Grootste toegestane diepte van het bewijs (bits in een u64-index); dezelfde
/// constante als in `tree`, waar `Proof::verify` hem gebruikt.
pub const MAX_DIEPTE: usize = crate::tree::MAX_DIEPTE;
/// Bytes tot en met het diepteveld.
const KOP: usize = 148;
/// Vaste bytes van een record; daarbij komt 64 × diepte.
pub const VASTE_BYTES: usize = 276;

/// Lengte van een record met een bewijs van `diepte` niveaus.
pub const fn record_bytes(diepte: usize) -> usize {
    VASTE_BYTES + DIGEST_BYTES * diepte
}

/// Waarom een record niet te lezen is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Formaat {
    Lengte,
    Versie,
    Schema,
    Niveau,
    Diepte,
}

/// Waarom een besteding geweigerd wordt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BestedingFout {
    /// Controle 1.
    Formaat(Formaat),
    /// Controle 2: de publieke sleutel is geen geldig punt.
    OngeldigeSleutel,
    /// Controle 4: de wortel valt niet in het venster.
    WortelOnbekend,
    /// Controle 5: geen opgenomen munt met deze opening op deze positie.
    GeenLid,
    /// Controle 6: deze munt op deze positie is al besteed.
    Dubbeluitgave,
    /// Controle 7.
    Handtekening,
    /// Controle 8: niets besteedbaar in de emmer van de invoer.
    LegeEmmer,
    /// Controle 8: meer bestedingen dan opnames; de tellers zijn beschadigd.
    NegatieveEmmer,
}

/// Een gelezen record. `to_bytes` en `from_bytes` zijn elkaars inverse voor
/// elk record met een diepte van ten hoogste 64 (`MAX_DIEPTE`); een dieper
/// record wordt bij het lezen geweigerd.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub versie: u8,
    pub schema: u8,
    pub index_in: u64,
    pub niveau: u8,
    pub epoch_in: u64,
    pub geheim_in: [u8; 32],
    pub pk: [u8; 32],
    pub wortel: Digest,
    pub bewijs: Proof,
    pub c_uit: Digest,
    pub handtekening: [u8; 64],
}

impl Record {
    /// Draadformaat. Ondersteund tot diepte 64 (`MAX_DIEPTE`); een dieper
    /// bewijs geeft een record dat `from_bytes` weigert (een diepte boven
    /// 255 wordt als 255 geschreven).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(record_bytes(self.bewijs.depth()));
        b.push(self.versie);
        b.push(self.schema);
        b.extend_from_slice(&self.index_in.to_le_bytes());
        b.push(self.niveau);
        b.extend_from_slice(&self.epoch_in.to_le_bytes());
        b.extend_from_slice(&self.geheim_in);
        b.extend_from_slice(&self.pk);
        b.extend_from_slice(&self.wortel);
        b.push(u8::try_from(self.bewijs.depth()).unwrap_or(u8::MAX));
        b.extend_from_slice(&self.bewijs.to_bytes());
        b.extend_from_slice(&self.c_uit);
        b.extend_from_slice(&self.handtekening);
        b
    }

    /// Controle 1. Leest zonder paniek, voor elke invoer.
    pub fn from_bytes(b: &[u8]) -> Result<Record, BestedingFout> {
        use BestedingFout::Formaat as F;
        if b.len() < KOP {
            return Err(F(Formaat::Lengte));
        } // [C1-kop]
        let versie = b[0];
        if versie != VERSIE {
            return Err(F(Formaat::Versie));
        } // [C1-versie]
        let schema = b[1];
        if schema != Ed25519::SCHEMA_ID {
            return Err(F(Formaat::Schema));
        } // [C1-schema]
        let niveau = b[10];
        if niveau as usize >= DENOM_LEVELS {
            return Err(F(Formaat::Niveau));
        } // [C1-niveau]
        let diepte = b[147] as usize;
        if diepte > MAX_DIEPTE {
            return Err(F(Formaat::Diepte));
        } // [C1-diepte]
        if b.len() != record_bytes(diepte) {
            return Err(F(Formaat::Lengte));
        } // [C1-lengte]

        let mut pos = 0usize;
        let mut neem = |n: usize| {
            let s = &b[pos..pos + n];
            pos += n;
            s
        };
        let _ = neem(2);
        let index_in = u64::from_le_bytes(neem(8).try_into().map_err(|_| F(Formaat::Lengte))?);
        let _ = neem(1);
        let epoch_in = u64::from_le_bytes(neem(8).try_into().map_err(|_| F(Formaat::Lengte))?);
        let geheim_in: [u8; 32] = neem(32).try_into().map_err(|_| F(Formaat::Lengte))?;
        let pk: [u8; 32] = neem(32).try_into().map_err(|_| F(Formaat::Lengte))?;
        let wortel: Digest = neem(64).try_into().map_err(|_| F(Formaat::Lengte))?;
        let _ = neem(1);
        let bewijs = Proof::from_bytes(neem(DIGEST_BYTES * diepte)).ok_or(F(Formaat::Lengte))?;
        let c_uit: Digest = neem(64).try_into().map_err(|_| F(Formaat::Lengte))?;
        let handtekening: [u8; 64] = neem(64).try_into().map_err(|_| F(Formaat::Lengte))?;
        Ok(Record {
            versie,
            schema,
            index_in,
            niveau,
            epoch_in,
            geheim_in,
            pk,
            wortel,
            bewijs,
            c_uit,
            handtekening,
        })
    }
}

/// `N = H("GL/nullify/v1", chain, index_in, C_in)`, index als u64 LE. De
/// positie zit erin: dezelfde commitment op twee posities zijn twee munten.
pub fn nullifier(chain: &ChainId, index_in: u64, c_in: &Digest) -> Digest {
    dsh(b"GL/nullify/v1", &[&chain.0, &index_in.to_le_bytes(), c_in])
}

/// Het getekende bericht: `dsh("GL/besteding/v1", [chain, versie, schema, N,
/// niveau, C_uit])`, 64 byte. Wortel en bewijs zijn niet getekend; de index
/// wel, via N.
pub fn bericht(
    chain: &ChainId,
    versie: u8,
    schema: u8,
    n: &Digest,
    niveau: u8,
    c_uit: &Digest,
) -> Digest {
    dsh(
        b"GL/besteding/v1",
        &[&chain.0, &[versie], &[schema], n, &[niveau], c_uit],
    )
}

/// Wat een geslaagde besteding oplevert.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Besteed {
    /// De nullifier die nu verbrand is.
    pub nullifier: Digest,
    /// Positie van de uitgang in de accumulator.
    pub index_uit: usize,
}

impl Ledger {
    /// Voert één zichtbare besteding uit. Controles 1 tot en met 8 lezen
    /// alleen; bij elke weigering is de ledger onveranderd.
    pub fn besteed(&mut self, record: &[u8]) -> Result<Besteed, BestedingFout> {
        use BestedingFout as B;
        // 1. Formaat.
        let r = Record::from_bytes(record)?;
        // 2. Sleutel inlezen.
        Ed25519::lees_sleutel(&r.pk).map_err(|_| B::OngeldigeSleutel)?; // [C2]

        // 3. Eigenaar en commitment van de invoer, door de keten uitgerekend.
        let owner = owner_hash(Ed25519::SCHEMA, &r.pk);
        let invoer = Note {
            level: r.niveau,
            epoch: r.epoch_in,
            owner,
            secret: Secret(r.geheim_in),
        };
        let c_in = invoer.commitment(&self.chain);
        // 4. Wortel in het venster.
        if !self.acc.accepts_root(&r.wortel) {
            return Err(B::WortelOnbekend);
        } // [C4]
          // 5. Lidmaatschap van blad(niveau, C_in) op index_in.
        let index = usize::try_from(r.index_in).map_err(|_| B::GeenLid)?;
        if !r.bewijs.verify(index, &blad(r.niveau, &c_in), &r.wortel) {
            return Err(B::GeenLid);
        } // [C5]
          // 6. Nullifier.
        let n = nullifier(&self.chain, r.index_in, &c_in); // [C6-n]
        if self.nullified.contains(&n) {
            return Err(B::Dubbeluitgave);
        } // [C6]
          // 7. Handtekening over exact het digest van 64 byte.
        let m = bericht(&self.chain, r.versie, r.schema, &n, r.niveau, &r.c_uit);
        Ed25519::verifieer(&r.pk, &m, &r.handtekening).map_err(|_| B::Handtekening)?; // [C7]

        // 8. Emmer van de invoer, en iets besteedbaars erin.
        let emmer = *self.minted.get(index).ok_or(B::GeenLid)?;
        if spend::live(&self.buckets, &self.spent, emmer.0, emmer.1)
            .map_err(|_| B::NegatieveEmmer)?
            == 0
        {
            return Err(B::LegeEmmer);
        } // [C8]
          // 9. Toepassen, in één keer.
        self.nullified.insert(n);
        *self.spent.entry(emmer).or_insert(0) += 1; // [C9-spent]
        let uit = (self.epoch, r.niveau); // [C9-emmer]
        let index_uit = self.acc.push(blad(r.niveau, &r.c_uit));
        self.minted.push(uit);
        *self.buckets.entry(uit).or_insert(0) += 1;
        Ok(Besteed {
            nullifier: n,
            index_uit,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spend::{live, menigte, mint_to_ledger};
    use ed25519_dalek::{Signer, SigningKey};

    fn keten() -> ChainId {
        ChainId([5u8; 32])
    }

    fn munt(sk: &SigningKey, level: u8, epoch: u64, k: u8) -> Note {
        let owner = owner_hash(Ed25519::SCHEMA, &sk.verifying_key().to_bytes());
        let mut s = [0u8; 32];
        s[0] = k;
        s[1] = level;
        s[2] = epoch as u8;
        s[31] = 0xA5;
        Note {
            level,
            epoch,
            owner,
            secret: Secret(s),
        }
    }

    fn record(l: &Ledger, sk: &SigningKey, n: &Note, index: usize, c_uit: Digest) -> Vec<u8> {
        let c_in = n.commitment(&l.chain);
        let nf = nullifier(&l.chain, index as u64, &c_in);
        let m = bericht(&l.chain, VERSIE, Ed25519::SCHEMA_ID, &nf, n.level, &c_uit);
        Record {
            versie: VERSIE,
            schema: Ed25519::SCHEMA_ID,
            index_in: index as u64,
            niveau: n.level,
            epoch_in: n.epoch,
            geheim_in: n.secret.0,
            pk: sk.verifying_key().to_bytes(),
            wortel: l.root(),
            bewijs: l.acc.proof(index).unwrap(),
            c_uit,
            handtekening: sk.sign(&m).to_bytes(),
        }
        .to_bytes()
    }

    /// Een kale commitment, zonder `blad()` in de boom gezet, is geen lid:
    /// de keten rekent het blad zelf uit. (Kan alleen binnen de crate worden
    /// opgezet; van buiten is `push` niet bereikbaar, zie stap 6.)
    #[test]
    fn kale_commitment_zonder_blad_is_geen_lid() {
        let mut l = Ledger::new(keten());
        let sk = SigningKey::from_bytes(&[1; 32]);
        let n = munt(&sk, 4, 0, 1);
        let i = l.acc.push(n.commitment(&l.chain));
        l.minted.push((0, 4));
        *l.buckets.entry((0, 4)).or_insert(0) += 1;
        let r = record(&l, &sk, &n, i, [9; 64]);
        let voor = format!("{l:?}");
        assert_eq!(l.besteed(&r), Err(BestedingFout::GeenLid));
        assert_eq!(format!("{l:?}"), voor);
    }

    /// Controle 8. Van buiten de crate is deze toestand niet te maken: sinds
    /// `Ledger::mint` de emmer telt, lopen boom en emmers niet uiteen. Hier
    /// wordt de inconsistentie via de `pub(crate)`-velden opgebouwd: een blad
    /// in de boom en in `minted`, maar niet in `buckets`. De munt is verder
    /// volledig geldig (lid, ongebruikt, correct getekend), dus alleen
    /// controle 8 kan haar weigeren, en de toestand blijft gelijk.
    #[test]
    fn controle_8_lege_emmer_bij_inconsistente_tellers() {
        let mut l = Ledger::new(keten());
        let sk = SigningKey::from_bytes(&[3; 32]);
        let n = munt(&sk, 4, 0, 1);
        let i = l.acc.push(blad(n.level, &n.commitment(&l.chain)));
        l.minted.push((0, 4));
        let r = record(&l, &sk, &n, i, [9; 64]);
        let voor = format!("{l:?}");
        assert_eq!(l.besteed(&r), Err(BestedingFout::LegeEmmer));
        assert_eq!(format!("{l:?}"), voor);

        // Meer bestedingen dan opnames: NegatieveEmmer, niet LegeEmmer.
        l.buckets.insert((0, 4), 1);
        l.spent.insert((0, 4), 2);
        let voor = format!("{l:?}");
        assert_eq!(l.besteed(&r), Err(BestedingFout::NegatieveEmmer));
        assert_eq!(format!("{l:?}"), voor);

        // Met kloppende tellers slaagt hetzelfde record.
        l.spent.clear();
        assert!(l.besteed(&r).is_ok());
    }

    /// Overgenomen uit de oude `spend`-tests met dezelfde verwachte waarden:
    /// een besteding laat de opnames in de emmer met 1 groeien (de uitgang)
    /// en het aantal besteedbare munten gelijk.
    #[test]
    fn besteden_groeit_de_opnames_houdt_de_live_gelijk() {
        let mut l = Ledger::new(keten());
        l.epoch = 3;
        let sk = SigningKey::from_bytes(&[2; 32]);
        for i in 0..10 {
            mint_to_ledger(&mut l, &munt(&sk, 8, 3, i));
        }
        assert_eq!(menigte(&l.buckets, 3, 8), 10);
        assert_eq!(live(&l.buckets, &l.spent, 3, 8), Ok(10));

        let r = record(
            &l,
            &sk,
            &munt(&sk, 8, 3, 2),
            2,
            munt(&sk, 8, 3, 77).commitment(&l.chain),
        );
        l.besteed(&r).unwrap();
        assert_eq!(
            menigte(&l.buckets, 3, 8),
            11,
            "opnames hadden met 1 moeten groeien"
        );
        assert_eq!(
            live(&l.buckets, &l.spent, 3, 8),
            Ok(10),
            "live moet gelijk blijven"
        );

        mint_to_ledger(&mut l, &munt(&sk, 8, 3, 78));
        assert_eq!(menigte(&l.buckets, 3, 8), 12);
        assert_eq!(live(&l.buckets, &l.spent, 3, 8), Ok(11));
    }
}
