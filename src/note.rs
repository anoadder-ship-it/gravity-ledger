//! De noot: het enige waarde-object in gravity-ledger.
//!
//! Een noot heeft **één vaste denominatie**: haar niveau. Er is geen bedrag
//! dat apart verborgen of begrensd moet worden; de noot *is* de waarde. Een
//! bewijs dat een verborgen bedrag binnen een bereik ligt (een range-proof) is
//! daarom niet nodig.
//!
//! Alle hashes in dit bestand zijn `dsh`: BLAKE3, uitgebreid tot 64 byte via
//! de XOF, met een eigen domeintag per gebruik (`src/hash.rs`). Wat hieronder
//! over binding en verborgenheid staat, zijn aannames over die hash en over
//! het geheim; deze repo bevat er geen bewijs voor.

use crate::hash::{dsh, Digest};

/// Denominatieniveaus: 2^0 … 2^32. Vast, wereldwijd, nooit veranderbaar.
pub const DENOM_LEVELS: usize = 33;

/// Waarde van één niveau, `2^level`. Alleen aangeroepen in de tests van dit
/// bestand; er is geen audit-laag. Geeft een paniek bij niveau 33 of hoger
/// (STATUS.md sectie 6, punt 13a).
pub fn denom_value(level: u8) -> u64 {
    assert!((level as usize) < DENOM_LEVELS, "niveau buiten bereik");
    1u64 << level
}

/// Identiteit van de keten (32 byte). Gaat de hash in van de commitment, van
/// beide nullifiers (`Note::nullifier` en `besteding::nullifier`), van het
/// getekende bericht (`besteding::bericht`) en van de audit-tag, zodat een
/// noot of record van de ene keten op een andere een andere waarde geeft.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChainId(pub [u8; 32]);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Secret(pub [u8; 32]);

/// Een noot in bezit van een portemonnee. Bij een besteding wordt de opening
/// zichtbaar; zie `besteding`.
#[derive(Clone, Copy, Debug)]
pub struct Note {
    pub level: u8,
    /// Epoch van de noot; zit in de commitment en bepaalt mee de emmer
    /// (epoch, niveau) waarin `Ledger::mint` hem telt.
    pub epoch: u64,
    /// Eigenaar: de hash van de publieke sleutel (`owner_hash`). De sleutel
    /// zelf staat niet in de noot en wordt pas bij besteding getoond.
    pub owner: Digest,
    pub secret: Secret,
}

/// `H("GL/eigenaar/v1", schema, publieke sleutel)`. Het schema (bijvoorbeeld
/// `b"ed25519"`) zit erin zodat dezelfde bytes onder twee handtekening-
/// schema's nooit dezelfde eigenaar zijn.
pub fn owner_hash(scheme: &[u8], public_key: &[u8]) -> Digest {
    dsh(b"GL/eigenaar/v1", &[scheme, public_key])
}

impl Note {
    /// `C = H("GL/commit/v1", chain_id, level, epoch, owner, secret)`
    ///
    /// Dit is wat op de keten gaat: 64 byte, `dsh` (BLAKE3-XOF) met domeintag
    /// `GL/commit/v1`.
    ///
    /// Binding (niemand vindt twee verschillende openingen met dezelfde C)
    /// steunt op de aanname dat deze hash botsingsvrij is. Verborgenheid (C
    /// verraadt de velden niet) steunt op de aanname dat de hash niets over
    /// zijn invoer prijsgeeft, en op een onvoorspelbaar geheim van 32 byte: de
    /// andere velden zijn te raden. De bibliotheek maakt het geheim niet zelf
    /// aan; de houder kiest het. Bij een zichtbare besteding staat het geheim
    /// in het record, dus vanaf dan is C niet meer verborgen. Deze repo bevat
    /// voor geen van beide eigenschappen een bewijs.
    /// v1 voegt de eigenaar toe (stap 4): `Ledger::besteed` rekent `owner`
    /// uit de publieke sleutel in het record (controle 3) en controleert de
    /// handtekening onder die sleutel (controle 7).
    pub fn commitment(&self, chain: &ChainId) -> Digest {
        dsh(
            b"GL/commit/v1",
            &[&chain.0, &[self.level], &self.epoch.to_le_bytes(), &self.owner, &self.secret.0],
        )
    }

    /// `N = H("GL/nullify/v0", chain_id, level, epoch, secret)`
    ///
    /// **Onderzoek, niet het protocol.** De zichtbare besteding (stap 7)
    /// gebruikt `besteding::nullifier` (v1, gebonden aan positie en
    /// commitment); deze v0 hoort bij het anonieme model (STATUS.md 3d).
    ///
    /// De gedachte erachter: waar CryptoNote een key image `I = x·Hp(P)` op een
    /// elliptische kromme gebruikt, is dit een hash over het geheim (`dsh`,
    /// BLAKE3-XOF, 64 byte, domeintag `GL/nullify/v0`). Hij is deterministisch
    /// uit keten, niveau, epoch en geheim, dus een tweede besteding van dezelfde noot geeft dezelfde
    /// N. Dat N zonder het geheim niet aan de commitment te koppelen is, steunt
    /// op dezelfde aannames als de verborgenheid van de commitment
    /// (onvoorspelbaar geheim, een hash die niets over zijn invoer prijsgeeft).
    /// Deze repo bevat daar geen bewijs voor, en geen analyse van de sterkte
    /// van deze constructie tegen een quantumcomputer.
    pub fn nullifier(&self, chain: &ChainId) -> Digest {
        dsh(
            b"GL/nullify/v0",
            &[&chain.0, &[self.level], &self.epoch.to_le_bytes(), &self.secret.0],
        )
    }

    /// Een **audit-tag**: `dsh` (BLAKE3-XOF, 64 byte, domeintag `GL/audit/v0`)
    /// over keten, de identiteit van een auditor, niveau, epoch en geheim.
    /// Uitrekenen vraagt het geheim; dat alleen de houder hem kan maken, steunt
    /// dus op een onvoorspelbaar geheim en op de aannames over de hash. Hoe een
    /// auditor een tag herkent, staat niet in de code: er is geen audit-laag,
    /// en deze functie wordt alleen in de tests van dit bestand aangeroepen.
    ///
    /// Randgeval: een zichtbaar bestedingsrecord bevat het geheim, het niveau
    /// en de epoch van de invoer (`besteding::Record`: `geheim_in`, `niveau`,
    /// `epoch_in`). Wie het record en de keten kent, kan dus voor die noot de
    /// audit-tag (voor elke auditoridentiteit) en de v0-nullifier
    /// (`Note::nullifier`) uitrekenen. Vanaf dat moment geldt "alleen de
    /// houder kan hem maken" niet meer.
    pub fn audit_tag(&self, chain: &ChainId, auditor: &[u8]) -> Digest {
        dsh(
            b"GL/audit/v0",
            &[&chain.0, auditor, &[self.level], &self.epoch.to_le_bytes(), &self.secret.0],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chain() -> ChainId {
        ChainId([7u8; 32])
    }

    fn note(level: u8) -> Note {
        Note { level, epoch: 41, owner: [0u8; 64], secret: Secret([0xA5 + level; 32]) }
    }

    #[test]
    fn commitment_is_deterministisch() {
        let c = chain();
        assert_eq!(note(3).commitment(&c), note(3).commitment(&c));
    }

    #[test]
    fn nullifier_is_deterministisch_en_verschilt_van_commitment() {
        let c = chain();
        let n = note(5);
        assert_eq!(n.nullifier(&c), n.nullifier(&c), "image moet deterministisch zijn");
        assert_ne!(n.commitment(&c), n.nullifier(&c));
    }

    #[test]
    fn elke_veld_wijzigt_beide_afbeeldingen() {
        let c = chain();
        let base = note(4);
        let mut other = base;
        other.level = 5;
        assert_ne!(base.commitment(&c), other.commitment(&c));

        let mut other = base;
        other.epoch = 42;
        assert_ne!(base.nullifier(&c), other.nullifier(&c));

        let mut other = base;
        other.secret = Secret([0x00; 32]);
        assert_ne!(base.commitment(&c), other.commitment(&c));
    }

    #[test]
    fn chain_id_bindt_aan_keten() {
        // Een noot van keten A mag nooit als dezelfde commitment op keten B gelden.
        let a = ChainId([1u8; 32]);
        let b = ChainId([2u8; 32]);
        assert_ne!(note(4).commitment(&a), note(4).commitment(&b));
    }

    #[test]
    fn audit_tag_alleen_voor_de_wettige_auditor() {
        let c = chain();
        let n = note(6);
        assert_eq!(n.audit_tag(&c, b"toezichthouder-x"), n.audit_tag(&c, b"toezichthouder-x"));
        assert_ne!(n.audit_tag(&c, b"toezichthouder-x"), n.audit_tag(&c, b"iedereen"));
    }

    #[test]
    fn denominaties_zijn_machten_van_twee() {
        assert_eq!(denom_value(0), 1);
        assert_eq!(denom_value(10), 1024);
        assert_eq!(denom_value(32), 4_294_967_296);
    }
}
