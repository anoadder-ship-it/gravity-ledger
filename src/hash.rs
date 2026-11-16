//! Hashlaag — één primitief, geen handgeschreven cryptografie.
//!
//! `blake3` 1.8.6 is de officiële implementatie door de BLAKE3-auteurs; een
//! onafhankelijke audit is niet vastgesteld. De keten-hash is BLAKE3 extended
//! naar **64 byte** via de XOF.
//!
//! **Waarom 64 byte.** De bekende kwantum-botsingsaanval van Brassard, Høyer
//! en Tapp (BHT) vindt bij een hash met n bit uitvoer een botsing met ongeveer
//! 2^(n/3) bevragingen. Voor n = 256 is dat ongeveer 2^85 (256/3 ≈ 85,3), voor
//! n = 512 ongeveer 2^171 (512/3 ≈ 170,7). 2^85 vonden we te weinig voor een
//! systeem waarin een botsing geld uit het niets betekent. Daarom 64 byte,
//! niet 32. Dit is een rekensom op een algemene grens van die ene aanval, geen
//! bewijs en geen meting; deze repo bevat geen veiligheidsbewijs.
//!
//! **Waarom één primitief.** Ik had hier eerst `BLAKE3-256 ‖ SHA3-512` staan
//! (96 byte) als "moet beide breken". Dat was onnodig: voor dezelfde rekensom
//! volstaat één functie met een bredere output, tegen 33%
//! kleinere bewijzen en één afhankelijkheid minder. Echte scheiding zit in
//! domeinscheiding (`dsh`), niet in het stapelen van hashes — de reële bugs in
//! dit soort systemen zitten in die scheiding, niet in de primitieven.

pub const DIGEST_BYTES: usize = 64;
pub type Digest = [u8; DIGEST_BYTES];

/// De keten-hash: BLAKE3-XOF, 64 byte output.
pub fn hash(data: &[u8]) -> Digest {
    let mut hasher = blake3::Hasher::new();
    hasher.update(data);
    let mut out = [0u8; DIGEST_BYTES];
    hasher.finalize_xof().fill(&mut out);
    out
}

/// BLAKE3-256 — alleen gebruikt om de XOF-aanroep te verifiëren tegen een
/// officiële known answer (zie tests): de XOF-output begint met die 32 bytes.
/// Staat onder `cfg(test)`: de keten zelf gebruikt uitsluitend de XOF-versie.
#[cfg(test)]
fn blake3_256(data: &[u8]) -> [u8; 32] {
    *blake3::hash(data).as_bytes()
}

/// Domeinscheiding. Zonder deze scheiding is een commitment uit één context
/// bruikbaar in een andere: de bekende aanval waarbij een hash voor de ene
/// context in een andere context bruikbaar is.
///
/// Codering: `"gravity-ledger/v1" ‖ 0xff ‖ len(tag) ‖ tag ‖ (len(deel) ‖ deel)*`,
/// lengtes als u64 little-endian. De lengte vóór de tag maakt (tag, delen)
/// eenduidig; in v0 ontbrak die, waardoor tag `t ‖ len ‖ d` zonder delen
/// dezelfde hash gaf als tag `t` met deel `d` (review 2026-11-10, G5).
pub fn dsh(tag: &[u8], parts: &[&[u8]]) -> Digest {
    let mut buf =
        Vec::with_capacity(64 + tag.len() + parts.iter().map(|p| 8 + p.len()).sum::<usize>());
    buf.extend_from_slice(b"gravity-ledger/v1");
    buf.push(0xff);
    buf.extend_from_slice(&(tag.len() as u64).to_le_bytes());
    buf.extend_from_slice(tag);
    for p in parts {
        buf.extend_from_slice(&(p.len() as u64).to_le_bytes());
        buf.extend_from_slice(p);
    }
    hash(&buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_is_64_bytes() {
        assert_eq!(DIGEST_BYTES, 64);
        assert_eq!(hash(b"x").len(), 64);
    }

    #[test]
    fn xof_klopt_met_officiele_known_answer() {
        // BLAKE3 is zo ontworpen dat de standaard 256-bit hash de eerste 32
        // bytes van de XOF-stroom is. Hiermee controleren we dat onze XOF-
        // aanroep werkelijk BLAKE3 doet — gewaardeerd tegen de officiële vector.
        assert_eq!(
            hex(&blake3_256(b"")),
            "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"
        );
        assert_eq!(
            &hash(b"")[..32],
            &blake3_256(b"")[..],
            "XOF-prefix wijkt af"
        );
    }

    #[test]
    fn domeinscheiding_werkt() {
        assert_ne!(dsh(b"commit", &[b"a"]), dsh(b"nullify", &[b"a"]));
        // Belangrijker: samenvoegen van delen mag geen zelfde hash geven.
        assert_ne!(dsh(b"x", &[b"ab"]), dsh(b"x", &[b"a", b"b"]));
    }

    #[test]
    fn hash_is_geen_lege_transformatie() {
        // Controle dat we niet per ongeluk de input teruggeven.
        assert_ne!(&hash(b"proef")[..], &b"proef"[..]);
    }

    fn hex(d: &[u8]) -> String {
        d.iter().map(|b| format!("{:02x}", b)).collect()
    }
}
