//! Stap 5: de trait `Handtekening` met Ed25519.
//!
//! Testvectoren letterlijk uit RFC 8032, sectie 7.1 (TEST 1, 2, 3 en
//! SHA(abc)). Bron: https://www.rfc-editor.org/rfc/rfc8032.txt, gelezen op
//! 2026-11-12, sha256 van het bestand
//! ed63657ff389301282b169b0abde9b5dd2c7e4d524fdfa5da6ff3094fc93c4c3.
//! De hex is met een script uit dat bestand gehaald, niet overgetypt.
//!
//! Elke negatieve test controleert eerst dat het ongewijzigde geval slaagt,
//! zodat een implementatie die alles weigert hier rood is.

use ed25519_dalek::{Signer, SigningKey};
use gravity_ledger::handtekening::{Ed25519, Handtekening, SigFout};
use sha2::{Digest as _, Sha512};

struct Vector {
    naam: &'static str,
    geheim: &'static str,
    pk: &'static str,
    bericht: &'static str,
    sig: &'static str,
}

const RFC8032_7_1: [Vector; 4] = [
    Vector { naam: "TEST 1", geheim: "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60", pk: "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a", bericht: "", sig: "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b" },
    Vector { naam: "TEST 2", geheim: "4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb", pk: "3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c", bericht: "72", sig: "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00" },
    Vector { naam: "TEST 3", geheim: "c5aa8df43f9f837bedb7442f31dcb7b166d38535076f094b85ce3a2e0b4458f7", pk: "fc51cd8e6218a1a38da47ed00230f0580816ed13ba3303ac5deb911548908025", bericht: "af82", sig: "6291d657deec24024827e69c3abe01a30ce548a284743a445e3680d7db5ac3ac18ff9b538d16f290ae67f760984dc6594a7c15e9716ed28dc027beceea1ec40a" },
    Vector { naam: "TEST SHA(abc)", geheim: "833fe62409237b9d62ec77587520911e9a759cec1d19755b7da901b96dca3d42", pk: "ec172b93ad5e563bf4932c70e1245034c35467ef2efd4d64ebf819683467e2bf", bericht: "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f", sig: "dc2a4459e7369633a52b1bf277839a00201009a3efbf3ecb69bea2186c26b58909351fc9ac90b3ecfdfbc7c66431e0303dca179c138ac17ad9bef1177331a704" },
];

/// De groepsorde ℓ = 2^252 + 27742317777372353535851937790883648493
/// (RFC 8032, sectie 5.1), little-endian.
const L: [u8; 32] = [
    0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde, 0x14,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10,
];

fn hex(s: &str) -> Vec<u8> {
    assert!(s.len() % 2 == 0, "oneven hex");
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

struct Bytes {
    pk: Vec<u8>,
    bericht: Vec<u8>,
    sig: Vec<u8>,
}

fn bytes(v: &Vector) -> Bytes {
    Bytes {
        pk: hex(v.pk),
        bericht: hex(v.bericht),
        sig: hex(v.sig),
    }
}

fn sha_abc() -> Bytes {
    bytes(&RFC8032_7_1[3])
}

/// Known-answer: de trait aanvaardt elke RFC-vector, en de bibliotheek leidt
/// uit het geheim precies de RFC-sleutel en -handtekening af.
#[test]
fn rfc8032_vectoren_via_de_trait() {
    for v in &RFC8032_7_1 {
        let b = bytes(v);
        assert_eq!(
            Ed25519::verifieer(&b.pk, &b.bericht, &b.sig),
            Ok(()),
            "{}: geldige handtekening geweigerd",
            v.naam
        );

        let geheim: [u8; 32] = hex(v.geheim).try_into().unwrap();
        let sk = SigningKey::from_bytes(&geheim);
        assert_eq!(
            sk.verifying_key().to_bytes().to_vec(),
            b.pk,
            "{}: afgeleide sleutel wijkt af",
            v.naam
        );
        assert_eq!(
            sk.sign(&b.bericht).to_bytes().to_vec(),
            b.sig,
            "{}: handtekening wijkt af",
            v.naam
        );
    }
}

/// Onafhankelijke controle op de overname: het bericht van TEST SHA(abc) is
/// SHA-512("abc"), zoals de naam zegt.
#[test]
fn bericht_van_sha_abc_is_sha512_van_abc() {
    assert_eq!(sha_abc().bericht, Sha512::digest(b"abc").to_vec());
}

/// Elke handtekening geldt alleen voor haar eigen sleutel en bericht.
#[test]
fn geen_kruisgeldigheid_tussen_vectoren() {
    for (i, a) in RFC8032_7_1.iter().enumerate() {
        for (j, b) in RFC8032_7_1.iter().enumerate() {
            let (a, b) = (bytes(a), bytes(b));
            let uitkomst = Ed25519::verifieer(&b.pk, &b.bericht, &a.sig);
            if i == j {
                assert_eq!(uitkomst, Ok(()), "vector {i} geweigerd");
            } else {
                assert!(
                    uitkomst.is_err(),
                    "handtekening {i} geldt voor sleutel en bericht {j}"
                );
            }
        }
    }
}

/// Lengtes moeten exact kloppen: niets afkappen, niets aanvullen.
#[test]
fn lengtes_exact() {
    let b = sha_abc();
    assert_eq!(Ed25519::verifieer(&b.pk, &b.bericht, &b.sig), Ok(()));

    for n in [0usize, 31, 33, 64] {
        let mut pk = b.pk.clone();
        pk.resize(n, 0);
        assert_eq!(
            Ed25519::verifieer(&pk, &b.bericht, &b.sig),
            Err(SigFout::PkLengte {
                verwacht: 32,
                kreeg: n
            }),
            "pk van {n} bytes"
        );
    }
    for n in [0usize, 32, 63, 65, 128] {
        let mut sig = b.sig.clone();
        sig.resize(n, 0);
        assert_eq!(
            Ed25519::verifieer(&b.pk, &b.bericht, &sig),
            Err(SigFout::SigLengte {
                verwacht: 64,
                kreeg: n
            }),
            "handtekening van {n} bytes"
        );
    }
}

/// Elke bitverdraaiing in handtekening (512), bericht (512) of sleutel (256)
/// maakt de handtekening ongeldig.
#[test]
fn elke_bitverdraaiing_faalt() {
    let b = sha_abc();
    assert_eq!(Ed25519::verifieer(&b.pk, &b.bericht, &b.sig), Ok(()));

    for bit in 0..b.sig.len() * 8 {
        let mut sig = b.sig.clone();
        sig[bit / 8] ^= 1 << (bit % 8);
        assert!(
            Ed25519::verifieer(&b.pk, &b.bericht, &sig).is_err(),
            "handtekening, bit {bit}"
        );
    }
    for bit in 0..b.bericht.len() * 8 {
        let mut m = b.bericht.clone();
        m[bit / 8] ^= 1 << (bit % 8);
        assert!(
            Ed25519::verifieer(&b.pk, &m, &b.sig).is_err(),
            "bericht, bit {bit}"
        );
    }
    for bit in 0..b.pk.len() * 8 {
        let mut pk = b.pk.clone();
        pk[bit / 8] ^= 1 << (bit % 8);
        assert!(
            Ed25519::verifieer(&pk, &b.bericht, &b.sig).is_err(),
            "sleutel, bit {bit}"
        );
    }
}

/// Een niet-canonieke S (S + ℓ) beschrijft dezelfde scalar en zou zonder
/// controle een tweede geldige handtekening opleveren. Die moet geweigerd
/// worden.
#[test]
fn niet_canonieke_s_faalt() {
    for v in &RFC8032_7_1 {
        let b = bytes(v);
        assert_eq!(
            Ed25519::verifieer(&b.pk, &b.bericht, &b.sig),
            Ok(()),
            "{}",
            v.naam
        );

        let mut sig = b.sig.clone();
        let mut carry = 0u16;
        for k in 0..32 {
            let s = sig[32 + k] as u16 + L[k] as u16 + carry;
            sig[32 + k] = s as u8;
            carry = s >> 8;
        }
        assert_eq!(carry, 0, "{}: S + ℓ past niet in 32 bytes", v.naam);
        assert!(
            Ed25519::verifieer(&b.pk, &b.bericht, &sig).is_err(),
            "{}: S + ℓ aanvaard",
            v.naam
        );
    }
}

/// Codering van het neutrale punt (x = 0, y = 1): een punt van kleine orde.
const NEUTRAAL: [u8; 32] = {
    let mut b = [0u8; 32];
    b[0] = 0x01;
    b
};

/// Sleutel van kleine orde. Met A = R = het neutrale punt en S = 0 geldt
/// [S]B = R + [k]A voor élk bericht: deze ene handtekening "tekent" alles.
/// Gewone `verify` aanvaardt haar; `verify_strict` moet haar weigeren.
#[test]
fn sleutel_van_kleine_orde_wordt_geweigerd() {
    let mut sig = [0u8; 64];
    sig[..32].copy_from_slice(&NEUTRAAL);
    for bericht in [
        &b"gravity-ledger: willekeurig vast bericht"[..],
        b"",
        &[0xAB; 64],
    ] {
        assert!(
            Ed25519::verifieer(&NEUTRAAL, bericht, &sig).is_err(),
            "handtekening onder het neutrale punt aanvaard voor {bericht:?}"
        );
    }
}

/// Kleinste n waarvoor de 32 bytes [n, 0, …, 0] geen punt op edwards25519
/// zijn: voor y = 2 heeft x² = (y² − 1)/(d·y² + 1) geen wortel mod p
/// (RFC 8032 §5.1.3; gevonden met een script, n = 0 en 1 decoderen wel).
const GEEN_PUNT: u8 = 2;

#[test]
fn geen_punt_op_de_kromme_geeft_ongeldige_sleutel() {
    let b = sha_abc();
    let mut pk = [0u8; 32];
    pk[0] = GEEN_PUNT;
    assert_eq!(
        Ed25519::verifieer(&pk, &b.bericht, &b.sig),
        Err(SigFout::OngeldigeSleutel)
    );
    // De kleinere waarden zijn wel punten: de fout komt dan uit de handtekening.
    for n in 0..GEEN_PUNT {
        pk[0] = n;
        assert_eq!(
            Ed25519::verifieer(&pk, &b.bericht, &b.sig),
            Err(SigFout::Ongeldig),
            "n = {n}"
        );
    }
}
