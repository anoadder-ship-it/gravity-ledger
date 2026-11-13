//! Veiligheidstests naar aanleiding van de externe review van 2026-11-10.
//! Elke test beschrijft het *gewenste* gedrag. De meeste tests waren rood vóór
//! hun reparatie; r4_behoud_omgekeerd en
//! stap4_eigenaarshash_scheidt_schema_en_sleutel zijn asserts zonder reparatie.
//! Het historische verslag, met de oorspronkelijke code van de review, staat
//! in `docs/review-2026-11-10.md`. R4 uit dat verslag staat hieronder als
//! assert; R5 staat in de unit-tests van `src/tree.rs`.

use gravity_ledger::conservation::Shape;
use gravity_ledger::hash::{dsh, hash};
use gravity_ledger::note::{owner_hash, DENOM_LEVELS};
use gravity_ledger::spend::mint_to_ledger;
use gravity_ledger::tree::{Proof, MAX_DIEPTE};
use gravity_ledger::{ChainId, Ledger, Note, Secret};

fn keten() -> ChainId { ChainId([3u8; 32]) }

fn noot(level: u8, epoch: u64, k: u8) -> Note {
    let mut s = [0u8; 32];
    s[0] = k; s[1] = level; s[2] = epoch as u8;
    Note { level, epoch, owner: [0u8; 64], secret: Secret(s) }
}

/// G4 / R3: correct getekende records voor munten in emmer (9,5) die nooit
/// zijn opgenomen, worden geweigerd. Sinds stap 7 gebeurt dat al bij
/// lidmaatschap (`GeenLid`, controle 5); de test eist alleen een weigering en
/// dat geen emmer negatief wordt. De emmercontrole zelf (controle 8) wordt
/// bewaakt door `controle_8_lege_emmer_bij_inconsistente_tellers` in
/// `src/besteding.rs`.
#[test]
fn g4_besteden_uit_lege_emmer_faalt() {
    use ed25519_dalek::{Signer, SigningKey};
    use gravity_ledger::besteding::{bericht, nullifier, Record, VERSIE};
    use gravity_ledger::handtekening::{Ed25519, Handtekening};
    let mut l = Ledger::new(keten());
    let sk = SigningKey::from_bytes(&[4; 32]);
    let pk = sk.verifying_key().to_bytes();
    for k in 0..3u8 {
        let mut n = noot(5, 9, k);
        n.owner = owner_hash(Ed25519::SCHEMA, &pk);
        let c_in = n.commitment(l.chain());
        let c_uit = noot(5, 9, 100 + k).commitment(l.chain());
        let nf = nullifier(l.chain(), k as u64, &c_in);
        let m = bericht(l.chain(), VERSIE, Ed25519::SCHEMA_ID, &nf, 5, &c_uit);
        let r = Record { versie: VERSIE, schema: Ed25519::SCHEMA_ID, index_in: k as u64, niveau: 5, epoch_in: 9,
                         geheim_in: n.secret.0, pk, wortel: l.root(), bewijs: Proof { path: vec![] }, c_uit,
                         handtekening: sk.sign(&m).to_bytes() };
        assert!(l.besteed(&r.to_bytes()).is_err(),
                "besteding {k} uit lege emmer (9,5) werd aanvaard");
    }
    for (emmer, n) in l.spent() {
        assert!(*n <= l.buckets().get(emmer).copied().unwrap_or(0), "emmer {emmer:?} negatief");
    }
}

/// Stap 3: het niveau staat publiek in het blad. Een lidmaatschapsbewijs voor
/// een noot op niveau 3 mag met alleen publieke gegevens (positie, niveau,
/// commitment, bewijs, wortel) voor geen enkel ander niveau slagen.
#[test]
fn stap3_niveau_staat_publiek_in_het_blad() {
    let mut l = Ledger::new(keten());
    for k in 0..5 { mint_to_ledger(&mut l, &noot(7, 1, k)); }
    let n = noot(3, 1, 9);
    let i = mint_to_ledger(&mut l, &n);
    let c = n.commitment(l.chain());
    let p = l.acc().proof(i).unwrap();
    let w = l.root();
    assert!(l.is_lid(&p, i, 3, &c, &w), "echte noot op haar eigen niveau geweigerd");
    for niveau in (0..DENOM_LEVELS as u8).filter(|&v| v != 3) {
        assert!(!l.is_lid(&p, i, niveau, &c, &w), "noot van niveau 3 bewijsbaar als niveau {niveau}");
    }
}

/// Stap 4: de commitment bindt de eigenaar. Twee noten die alleen in de hash
/// van de publieke sleutel verschillen, hebben verschillende commitments en
/// dus verschillende bladen; een bewijs voor de ene geldt niet voor de andere.
#[test]
fn stap4_commitment_bindt_de_eigenaar() {
    let mut l = Ledger::new(keten());
    let mut van_a = noot(6, 2, 1);
    van_a.owner = owner_hash(b"ed25519", &[0xAA; 32]);
    let mut van_b = van_a;
    van_b.owner = owner_hash(b"ed25519", &[0xBB; 32]);
    assert_ne!(van_a.commitment(l.chain()), van_b.commitment(l.chain()), "eigenaar zit niet in de commitment");

    let i = mint_to_ledger(&mut l, &van_a);
    let p = l.acc().proof(i).unwrap();
    let w = l.root();
    assert!(l.is_lid(&p, i, 6, &van_a.commitment(l.chain()), &w));
    assert!(!l.is_lid(&p, i, 6, &van_b.commitment(l.chain()), &w), "bewijs van A geldt voor B");
}

/// De eigenaarshash scheidt schema's en sleutels.
#[test]
fn stap4_eigenaarshash_scheidt_schema_en_sleutel() {
    assert_ne!(owner_hash(b"ed25519", &[1; 32]), owner_hash(b"fn-dsa-512", &[1; 32]));
    assert_ne!(owner_hash(b"ed25519", &[1; 32]), owner_hash(b"ed25519", &[2; 32]));
}

/// Stap 6: het bewijs is niet vervormbaar. De richting op elk niveau mag niet
/// vrij te kiezen zijn: geen enkele bitverdraaiing in de gecodeerde vorm van
/// een echt bewijs mag nog een aanvaard bewijs opleveren. Met een vlag in het
/// formaat faalt dit op het losse kind aan de rechterrand van een oneven
/// niveau, waar beide richtingen dezelfde ouder geven.
#[test]
fn stap6_bewijs_is_niet_vervormbaar() {
    for n in 1..=20u8 {
        let mut l = Ledger::new(keten());
        for k in 0..n { mint_to_ledger(&mut l, &noot(4, 1, k)); }
        let w = l.root();
        for i in 0..n as usize {
            let c = noot(4, 1, i as u8).commitment(l.chain());
            let echt = l.acc().proof(i).unwrap();
            assert!(l.is_lid(&echt, i, 4, &c, &w), "n={n} i={i}: echt bewijs geweigerd");
            let bytes = echt.to_bytes();
            assert_eq!(Proof::from_bytes(&bytes).as_ref(), Some(&echt), "n={n} i={i}: codering is niet omkeerbaar");
            for bit in 0..bytes.len() * 8 {
                let mut b = bytes.clone();
                b[bit / 8] ^= 1 << (bit % 8);
                if let Some(q) = Proof::from_bytes(&b) {
                    assert!(!l.is_lid(&q, i, 4, &c, &w), "n={n} i={i}: bit {bit} verdraaid en toch aanvaard");
                }
            }
        }
    }
}

/// Een bewijs dat langer is dan een index bits heeft, wordt geweigerd zonder
/// paniek (de richting op niveau k is bit k van de index).
#[test]
fn stap6_te_lang_bewijs_wordt_geweigerd() {
    let mut l = Ledger::new(keten());
    let n = noot(4, 1, 0);
    mint_to_ledger(&mut l, &n);
    let c = n.commitment(l.chain());
    mint_to_ledger(&mut l, &noot(4, 1, 1));
    let w = l.root();
    let p = l.acc().proof(0).unwrap();
    assert!(l.is_lid(&p, 0, 4, &c, &w));
    assert_eq!(p.depth(), 1);
    // Bytes per niveau, ongeacht het formaat; nulbytes zijn in elk formaat
    // een geldige codering.
    let per_niveau = p.to_bytes().len();
    let lang = Proof::from_bytes(&vec![0u8; per_niveau * (MAX_DIEPTE + 1)]).unwrap();
    assert_eq!(lang.depth(), MAX_DIEPTE + 1);
    assert!(!l.is_lid(&lang, 0, 4, &c, &w), "bewijs van {} niveaus aanvaard", lang.depth());
}

/// De codering van `dsh`, met de hand uitgeschreven, los van de implementatie:
/// `"gravity-ledger/v1" ‖ 0xff ‖ len(tag) ‖ tag ‖ (len(deel) ‖ deel)*`,
/// lengtes als u64 little-endian.
fn dsh_volgens_spec(tag: &[u8], parts: &[&[u8]]) -> [u8; 64] {
    let mut b = b"gravity-ledger/v1".to_vec();
    b.push(0xff);
    b.extend_from_slice(&(tag.len() as u64).to_le_bytes());
    b.extend_from_slice(tag);
    for p in parts {
        b.extend_from_slice(&(p.len() as u64).to_le_bytes());
        b.extend_from_slice(p);
    }
    hash(&b)
}

/// G5 / R6: een tag mag niet door te verlengen overlopen in de delen.
#[test]
fn g5_tag_en_delen_zijn_eenduidig() {
    let a = dsh(b"x", &[b"abcdefgh12345678"]);
    let mut t = b"x".to_vec();
    t.extend_from_slice(&16u64.to_le_bytes());
    t.extend_from_slice(b"abcdefgh12345678");
    assert_ne!(a, dsh(&t, &[]), "(tag, delen) is niet eenduidig gecodeerd");
}

/// G5: `dsh` volgt precies de vastgelegde codering, inclusief lege tag en
/// lege delen.
#[test]
fn g5_dsh_volgt_de_codering() {
    let gevallen: [(&[u8], &[&[u8]]); 4] = [
        (b"GL/commit/v0", &[b"a", b"bc"]),
        (b"", &[b"x"]),
        (b"x", &[]),
        (b"GL/node/v0", &[b"", b""]),
    ];
    for (tag, parts) in gevallen {
        assert_eq!(dsh(tag, parts), dsh_volgens_spec(tag, parts), "tag {:?}", tag);
    }
}

/// R4: omgekeerde richting van de behoudstest. Als `check()` een vorm
/// aanvaardt, zijn de sommen gelijk (2.000.000 pseudo-willekeurige vormen,
/// vaste xorshift-zaad, dus reproduceerbaar). Plus de randen op niveau 32.
#[test]
fn r4_behoud_omgekeerd() {
    let mut x: u64 = 0x9E3779B97F4A7C15;
    let mut nx = |m: u64| { x ^= x << 13; x ^= x >> 7; x ^= x << 17; x % m };
    let (mut ok, mut fout) = (0u32, 0u32);
    for _ in 0..2_000_000 {
        let mut s = Shape::default();
        let lo = nx(31) as usize;
        for d in lo..lo + 3 {
            s.ins[d] = nx(3) as u32; s.outs[d] = nx(3) as u32; s.fee[d] = nx(2) as u32;
            s.splits[d] = nx(2) as u32; s.merges[d] = nx(2) as u32;
        }
        if s.check().is_ok() {
            ok += 1;
            if Shape::total_value(&s.ins) != Shape::total_value(&s.outs) + Shape::total_value(&s.fee) { fout += 1; }
        }
    }
    assert!(ok > 0, "geen enkele geldige vorm gegenereerd: de test toetst niets");
    assert_eq!(fout, 0, "{fout} van {ok} aanvaarde vormen hebben ongelijke sommen");
    let mut s = Shape::default(); s.ins[32] = 1; s.splits[32] = 1; s.outs[31] = 2;
    assert!(s.check().is_ok(), "splitsen op niveau 32 geweigerd");
    let mut s = Shape::default(); s.merges[32] = 1; s.ins[31] = 2; s.outs[32] = 1;
    assert!(s.check().is_ok(), "samenvoegen naar niveau 32 geweigerd");
}

// R5 (boom en venster) staat sinds stap 6 als `r5_boom_en_venster` in de
// unit-tests van src/tree.rs: hij bouwt een losse accumulator met willekeurige
// bladen, en `Accumulator::push` is van buiten de crate niet meer bereikbaar.
// Bladen, asserts en verwachte uitkomsten zijn ongewijzigd.
