//! Stap 7: de zichtbare, getekende besteding.
//!
//! Elke test eist een exacte fout en controleert dat de ledger bij een
//! weigering byte-gelijk blijft (via de volledige Debug-weergave: wortel,
//! venster, nullifiers, emmers, spent, minted en lengte). Welke controle bij
//! weghalen welke test rood maakt, staat in de mutatietabel van STATUS.md
//! (sectie 0a).

use ed25519_dalek::{Signer, SigningKey};
use gravity_ledger::besteding::{
    bericht, nullifier, record_bytes, BestedingFout as F, Formaat, Record, MAX_DIEPTE, VERSIE,
};
use gravity_ledger::handtekening::{Ed25519, Handtekening};
use gravity_ledger::hash::{dsh, Digest};
use gravity_ledger::note::owner_hash;
use gravity_ledger::spend::{live, mint_to_ledger};
use gravity_ledger::tree::Proof;
use gravity_ledger::{ChainId, Ledger, Note, Secret};

fn keten() -> ChainId { ChainId([7u8; 32]) }

fn sleutel(k: u8) -> SigningKey { SigningKey::from_bytes(&[k; 32]) }

fn pk(s: &SigningKey) -> [u8; 32] { s.verifying_key().to_bytes() }

/// Een munt van de eigenaar van `s`.
fn munt(s: &SigningKey, level: u8, epoch: u64, k: u8) -> Note {
    let mut g = [0u8; 32];
    g[0] = k; g[1] = level; g[2] = epoch as u8; g[3] = s.to_bytes()[0]; g[31] = 0x5A;
    Note { level, epoch, owner: owner_hash(Ed25519::SCHEMA, &pk(s)), secret: Secret(g) }
}

/// Commitment van een uitgang voor eigenaar 200.
fn uit(l: &Ledger, level: u8, k: u8) -> Digest {
    munt(&sleutel(200), level, l.epoch(), k).commitment(l.chain())
}

/// Record met actuele wortel en bewijs, nog niet getekend.
fn kaal(l: &Ledger, s: &SigningKey, n: &Note, index: usize, c_uit: Digest) -> Record {
    Record {
        versie: VERSIE, schema: Ed25519::SCHEMA_ID, index_in: index as u64, niveau: n.level,
        epoch_in: n.epoch, geheim_in: n.secret.0, pk: pk(s), wortel: l.root(),
        bewijs: l.acc().proof(index).unwrap_or(Proof { path: vec![] }), c_uit, handtekening: [0; 64],
    }
}

/// Tekent een record zoals een eerlijke portemonnee: C_in uit de velden van
/// het record, dan N en M met de functies van de bibliotheek.
fn teken(chain: &ChainId, s: &SigningKey, r: &mut Record) {
    let n = nullifier(chain, r.index_in, &c_in(chain, r));
    let m = bericht(chain, r.versie, r.schema, &n, r.niveau, &r.c_uit);
    r.handtekening = s.sign(&m).to_bytes();
}

fn c_in(chain: &ChainId, r: &Record) -> Digest {
    Note { level: r.niveau, epoch: r.epoch_in, owner: owner_hash(Ed25519::SCHEMA, &r.pk), secret: Secret(r.geheim_in) }
        .commitment(chain)
}

fn geldig(l: &Ledger, s: &SigningKey, n: &Note, index: usize, c_uit: Digest) -> Record {
    let mut r = kaal(l, s, n, index, c_uit);
    teken(l.chain(), s, &mut r);
    r
}

fn toestand(l: &Ledger) -> String { format!("{l:?}") }

/// Eist precies `fout` en een onveranderde ledger.
fn weigert(l: &mut Ledger, r: &[u8], fout: F) {
    let voor = toestand(l);
    assert_eq!(l.besteed(r), Err(fout));
    assert_eq!(toestand(l), voor, "ledger gewijzigd bij weigering {fout:?}");
}

// ---- R1, R2 ---------------------------------------------------------------

/// R1: een munt die nooit is uitgegeven, in een emmer die niet leeg is, met
/// het bewijs van de echte munt op die positie.
#[test]
fn r1_nooit_uitgegeven_munt() {
    let mut l = Ledger::new(keten());
    let a = sleutel(1);
    mint_to_ledger(&mut l, &munt(&a, 32, 0, 1));
    let nep = munt(&a, 32, 0, 99);
    let r = geldig(&l, &a, &nep, 0, uit(&l, 32, 1));
    weigert(&mut l, &r.to_bytes(), F::GeenLid);
    // De echte munt blijft gewoon te besteden.
    let echt = geldig(&l, &a, &munt(&a, 32, 0, 1), 0, uit(&l, 32, 2));
    assert!(l.besteed(&echt.to_bytes()).is_ok());
}

/// R2: hetzelfde geheim met een ander epoch is een andere commitment en dus
/// geen lid, ook als de emmer van dat epoch niet leeg is.
#[test]
fn r2_zelfde_geheim_ander_epoch() {
    let mut l = Ledger::new(keten());
    let a = sleutel(2);
    let echt = munt(&a, 10, 0, 7);
    mint_to_ledger(&mut l, &echt);
    mint_to_ledger(&mut l, &munt(&sleutel(3), 10, 1, 50));
    assert!(l.besteed(&geldig(&l, &a, &echt, 0, uit(&l, 10, 1)).to_bytes()).is_ok());
    let mut vals = echt;
    vals.epoch = 1;
    let r = geldig(&l, &a, &vals, 0, uit(&l, 10, 2));
    weigert(&mut l, &r.to_bytes(), F::GeenLid);
}

// ---- lidmaatschap ---------------------------------------------------------

#[test]
fn ander_niveau_in_de_opening() {
    let mut l = Ledger::new(keten());
    let a = sleutel(4);
    let n = munt(&a, 5, 0, 1);
    mint_to_ledger(&mut l, &n);
    let mut vals = n;
    vals.level = 6;
    let r = geldig(&l, &a, &vals, 0, uit(&l, 6, 1));
    weigert(&mut l, &r.to_bytes(), F::GeenLid);
}

#[test]
fn verkeerde_index() {
    let mut l = Ledger::new(keten());
    let a = sleutel(5);
    let n0 = munt(&a, 3, 0, 0);
    mint_to_ledger(&mut l, &n0);
    mint_to_ledger(&mut l, &munt(&a, 3, 0, 1));
    // Opening van munt 0, maar index en bewijs van positie 1; correct getekend.
    let mut r = kaal(&l, &a, &n0, 1, uit(&l, 3, 1));
    teken(l.chain(), &a, &mut r);
    weigert(&mut l, &r.to_bytes(), F::GeenLid);
}

/// Een vreemde sleutel met een geldige handtekening: de opening van het
/// slachtoffer, maar de eigen pk. De eigenaar zit in C, dus geen lid.
#[test]
fn vreemde_sleutel_met_geldige_handtekening() {
    let mut l = Ledger::new(keten());
    let slachtoffer = sleutel(6);
    let aanvaller = sleutel(66);
    let n = munt(&slachtoffer, 9, 0, 1);
    mint_to_ledger(&mut l, &n);
    let mut r = kaal(&l, &aanvaller, &n, 0, uit(&l, 9, 1));
    teken(l.chain(), &aanvaller, &mut r);
    assert!(Ed25519::verifieer(&r.pk, &bericht(l.chain(), r.versie, r.schema,
        &nullifier(l.chain(), 0, &c_in(l.chain(), &r)), r.niveau, &r.c_uit), &r.handtekening).is_ok(),
        "de handtekening van de aanvaller hoort op zich geldig te zijn");
    weigert(&mut l, &r.to_bytes(), F::GeenLid);
}

// ---- venster --------------------------------------------------------------

#[test]
fn wortel_buiten_het_venster() {
    let mut l = Ledger::new(keten());
    let a = sleutel(7);
    let n = munt(&a, 2, 0, 1);
    mint_to_ledger(&mut l, &n);
    let r = geldig(&l, &a, &n, 0, uit(&l, 2, 1));
    for k in 0..64u8 { mint_to_ledger(&mut l, &munt(&sleutel(8), 2, 0, k)); }
    weigert(&mut l, &r.to_bytes(), F::WortelOnbekend);
    // Met een actuele wortel en bewijs (ongetekend) gaat dezelfde handtekening wel.
    let mut vers = r.clone();
    vers.wortel = l.root();
    vers.bewijs = l.acc().proof(0).unwrap();
    assert!(l.besteed(&vers.to_bytes()).is_ok());
}

// ---- dubbele besteding ----------------------------------------------------

#[test]
fn hetzelfde_record_twee_keer() {
    let mut l = Ledger::new(keten());
    let a = sleutel(9);
    let n = munt(&a, 4, 0, 1);
    mint_to_ledger(&mut l, &n);
    mint_to_ledger(&mut l, &munt(&a, 4, 0, 2));
    let r = geldig(&l, &a, &n, 0, uit(&l, 4, 1)).to_bytes();
    let b = l.besteed(&r).unwrap();
    assert!(l.is_verbrand(&b.nullifier));
    weigert(&mut l, &r, F::Dubbeluitgave);
}

/// Dezelfde C op twee posities, in twee verschillende emmers: elke positie is
/// precies één keer te besteden en telt in de emmer van die positie.
#[test]
fn zelfde_c_op_twee_posities() {
    let mut l = Ledger::new(keten());
    let a = sleutel(10);
    let n = munt(&a, 6, 5, 1); // positie 0, emmer (5, 6)
    mint_to_ledger(&mut l, &n);
    let m = munt(&a, 6, 0, 2); // positie 1, emmer (0, 6)
    mint_to_ledger(&mut l, &m);
    // m besteden met als uitgang precies C(n): positie 2, emmer (0, 6).
    let c_n = n.commitment(l.chain());
    let b = l.besteed(&geldig(&l, &a, &m, 1, c_n).to_bytes()).unwrap();
    assert_eq!(b.index_uit, 2);
    assert_eq!(l.bucket_of(2), Some((0, 6)));
    assert_eq!(l.spent().get(&(0, 6)), Some(&1));

    let op0 = geldig(&l, &a, &n, 0, uit(&l, 6, 1)).to_bytes();
    assert!(l.besteed(&op0).is_ok(), "positie 0 niet te besteden");
    assert_eq!(l.spent().get(&(5, 6)), Some(&1));
    assert_eq!(l.spent().get(&(0, 6)), Some(&1));

    let op2 = geldig(&l, &a, &n, 2, uit(&l, 6, 2)).to_bytes();
    assert!(l.besteed(&op2).is_ok(), "positie 2 niet te besteden");
    assert_eq!(l.spent().get(&(5, 6)), Some(&1));
    assert_eq!(l.spent().get(&(0, 6)), Some(&2));

    weigert(&mut l, &op0, F::Dubbeluitgave);
    weigert(&mut l, &op2, F::Dubbeluitgave);
    // Ook met een vers bewijs blijven beide posities besteed.
    let vers0 = geldig(&l, &a, &n, 0, uit(&l, 6, 3)).to_bytes();
    let vers2 = geldig(&l, &a, &n, 2, uit(&l, 6, 4)).to_bytes();
    weigert(&mut l, &vers0, F::Dubbeluitgave);
    weigert(&mut l, &vers2, F::Dubbeluitgave);
}

/// Alleen index_in gewijzigd naar een andere positie met dezelfde C (met het
/// bijbehorende, ongetekende bewijs): de index zit via N in de handtekening.
#[test]
fn alleen_index_gewijzigd_naar_zelfde_c() {
    let mut l = Ledger::new(keten());
    let a = sleutel(11);
    let n = munt(&a, 7, 0, 1);
    mint_to_ledger(&mut l, &n);
    mint_to_ledger(&mut l, &n);
    let mut r = geldig(&l, &a, &n, 0, uit(&l, 7, 1));
    r.index_in = 1;
    r.bewijs = l.acc().proof(1).unwrap();
    weigert(&mut l, &r.to_bytes(), F::Handtekening);
}

// ---- handtekening ---------------------------------------------------------

#[test]
fn c_uit_gewijzigd_na_tekenen() {
    let mut l = Ledger::new(keten());
    let a = sleutel(12);
    let n = munt(&a, 4, 0, 1);
    mint_to_ledger(&mut l, &n);
    let mut r = geldig(&l, &a, &n, 0, uit(&l, 4, 1));
    r.c_uit = uit(&l, 4, 2);
    weigert(&mut l, &r.to_bytes(), F::Handtekening);
}

/// Getekend voor een ander niveau dan in het record staat (bij een opening
/// die wel lid is).
#[test]
fn niveau_gewijzigd_na_tekenen() {
    let mut l = Ledger::new(keten());
    let a = sleutel(13);
    let n = munt(&a, 4, 0, 1);
    mint_to_ledger(&mut l, &n);
    let mut r = kaal(&l, &a, &n, 0, uit(&l, 4, 1));
    let nf = nullifier(l.chain(), 0, &n.commitment(l.chain()));
    r.handtekening = a.sign(&bericht(l.chain(), VERSIE, Ed25519::SCHEMA_ID, &nf, 5, &r.c_uit)).to_bytes();
    weigert(&mut l, &r.to_bytes(), F::Handtekening);
}

/// Getekend met een andere keten in M (N en al het andere gelijk).
#[test]
fn chain_gewijzigd_na_tekenen() {
    let mut l = Ledger::new(keten());
    let a = sleutel(14);
    let n = munt(&a, 4, 0, 1);
    mint_to_ledger(&mut l, &n);
    let mut r = kaal(&l, &a, &n, 0, uit(&l, 4, 1));
    let nf = nullifier(l.chain(), 0, &n.commitment(l.chain()));
    r.handtekening = a.sign(&bericht(&ChainId([8u8; 32]), VERSIE, Ed25519::SCHEMA_ID, &nf, 4, &r.c_uit)).to_bytes();
    weigert(&mut l, &r.to_bytes(), F::Handtekening);
}

#[test]
fn elke_bit_van_de_handtekening() {
    let mut l = Ledger::new(keten());
    let a = sleutel(15);
    let n = munt(&a, 4, 0, 1);
    mint_to_ledger(&mut l, &n);
    let r = geldig(&l, &a, &n, 0, uit(&l, 4, 1));
    for bit in 0..512 {
        let mut q = r.clone();
        q.handtekening[bit / 8] ^= 1 << (bit % 8);
        weigert(&mut l, &q.to_bytes(), F::Handtekening);
    }
    assert!(l.besteed(&r.to_bytes()).is_ok(), "het ongewijzigde record hoort te slagen");
}

/// Een record van keten A op keten B, met dezelfde munt op dezelfde positie.
#[test]
fn record_van_keten_a_op_keten_b() {
    let a = sleutel(16);
    let n = munt(&a, 4, 0, 1);
    let mut la = Ledger::new(ChainId([0xA; 32]));
    let mut lb = Ledger::new(ChainId([0xB; 32]));
    mint_to_ledger(&mut la, &n);
    mint_to_ledger(&mut lb, &n);
    let r = geldig(&la, &a, &n, 0, uit(&la, 4, 1));
    weigert(&mut lb, &r.to_bytes(), F::WortelOnbekend);
    // Wortel en bewijs ververst voor B (ongetekend): dan op de handtekening.
    let mut q = r.clone();
    q.wortel = lb.root();
    q.bewijs = lb.acc().proof(0).unwrap();
    weigert(&mut lb, &q.to_bytes(), F::Handtekening);
    assert!(la.besteed(&r.to_bytes()).is_ok(), "op keten A hoort het record te slagen");
}

/// M is exact het digest van 64 byte uit de specificatie; met de hand
/// uitgeschreven, los van `nullifier` en `bericht`, en rechtstreeks met
/// ed25519-dalek over die 64 byte getekend.
#[test]
fn bericht_volgens_specificatie() {
    let mut l = Ledger::new(keten());
    let a = sleutel(17);
    let n = munt(&a, 11, 0, 1);
    mint_to_ledger(&mut l, &n);
    mint_to_ledger(&mut l, &munt(&a, 11, 0, 2));
    let index: u64 = 0;
    let c_in = n.commitment(l.chain());
    let c_uit = uit(&l, 11, 1);
    let n_spec: Digest = dsh(b"GL/nullify/v1", &[&keten().0, &index.to_le_bytes(), &c_in]);
    let m_spec: Digest = dsh(b"GL/besteding/v1", &[&keten().0, &[1u8], &[1u8], &n_spec, &[11u8], &c_uit]);
    assert_eq!(nullifier(l.chain(), index, &c_in), n_spec);
    assert_eq!(bericht(l.chain(), 1, 1, &n_spec, 11, &c_uit), m_spec);
    let mut r = kaal(&l, &a, &n, 0, c_uit);
    r.handtekening = a.sign(&m_spec).to_bytes();
    let b = l.besteed(&r.to_bytes()).expect("record getekend volgens de specificatie geweigerd");
    assert_eq!(b.nullifier, n_spec);
}

// ---- sleutel en emmer -----------------------------------------------------

/// De kleinste waarde [n, 0, …] die geen punt op de kromme is (zie
/// tests/handtekening.rs, GEEN_PUNT).
const GEEN_PUNT: u8 = 2;

#[test]
fn ongeldige_sleutel() {
    let mut l = Ledger::new(keten());
    let mut slecht = [0u8; 32];
    slecht[0] = GEEN_PUNT;
    let n = Note { level: 4, epoch: 0, owner: owner_hash(Ed25519::SCHEMA, &slecht), secret: Secret([3; 32]) };
    mint_to_ledger(&mut l, &n);
    let mut r = kaal(&l, &sleutel(18), &n, 0, uit(&l, 4, 1));
    r.pk = slecht;
    teken(l.chain(), &sleutel(18), &mut r);
    weigert(&mut l, &r.to_bytes(), F::OngeldigeSleutel);
}

/// Omkering van het vroegere `lege_emmer`: een munt die via `Ledger::mint`
/// is opgenomen, telt in zijn emmer en is gewoon te besteden. Controle 8
/// zelf (LegeEmmer) wordt bewaakt door de unit-test
/// `controle_8_lege_emmer_bij_inconsistente_tellers` in `src/besteding.rs`.
#[test]
fn ledger_mint_telt_in_de_emmer() {
    let mut l = Ledger::new(keten());
    let a = sleutel(19);
    let n = munt(&a, 4, 0, 1);
    l.mint(&n);
    assert_eq!(l.buckets().get(&(0, 4)), Some(&1));
    let r = geldig(&l, &a, &n, 0, uit(&l, 4, 1));
    assert!(l.besteed(&r.to_bytes()).is_ok());
    assert_eq!(live(l.buckets(), l.spent(), 0, 4), Ok(1));
}

/// Review van stap 7: een munt via `Ledger::mint` mocht de plek van een munt
/// via `mint_to_ledger` in dezelfde emmer niet opeten. Beide zijn te
/// besteden, in beide bestedingsvolgordes, en het aantal besteedbare munten
/// per emmer klopt na elke stap.
#[test]
fn twee_mintwegen_blokkeren_elkaar_niet() {
    for eerst_ander_besteden in [true, false] {
        let mut l = Ledger::new(keten());
        let a = sleutel(24);
        let h = sleutel(25);
        let eerlijk = munt(&a, 4, 5, 1);
        let ander = munt(&h, 4, 5, 2);
        mint_to_ledger(&mut l, &eerlijk);
        l.mint(&ander);
        assert_eq!(live(l.buckets(), l.spent(), 5, 4), Ok(2), "beide munten tellen in (5,4)");
        let ra = |l: &Ledger| geldig(l, &a, &eerlijk, 0, uit(l, 4, 1)).to_bytes();
        let rh = |l: &Ledger| geldig(l, &h, &ander, 1, uit(l, 4, 2)).to_bytes();
        if eerst_ander_besteden {
            let r = rh(&l); assert!(l.besteed(&r).is_ok());
            assert_eq!(live(l.buckets(), l.spent(), 5, 4), Ok(1));
            let r = ra(&l); assert!(l.besteed(&r).is_ok(), "eerlijke munt bevroren");
        } else {
            let r = ra(&l); assert!(l.besteed(&r).is_ok());
            assert_eq!(live(l.buckets(), l.spent(), 5, 4), Ok(1));
            let r = rh(&l); assert!(l.besteed(&r).is_ok(), "munt via Ledger::mint bevroren");
        }
        assert_eq!(live(l.buckets(), l.spent(), 5, 4), Ok(0));
        assert_eq!(live(l.buckets(), l.spent(), 0, 4), Ok(2));
    }
}

/// Twaalf munten over verschillende emmers (vijf niveaus, vier epochs, drie
/// eigenaars), via `Ledger::mint` en via `mint_to_ledger`: dezelfde posities
/// en een byte-gelijke ledger (wortel, venster, emmers, minted, lengte).
#[test]
fn mint_en_mint_to_ledger_geven_dezelfde_ledger() {
    let mut via_mint = Ledger::new(keten());
    let mut via_alias = Ledger::new(keten());
    for k in 0..12u8 {
        let n = munt(&sleutel(70 + k % 3), k % 5, (k % 4) as u64, k);
        assert_eq!(via_mint.mint(&n), mint_to_ledger(&mut via_alias, &n), "positie {k}");
    }
    assert_eq!(via_mint.buckets().len(), 12, "twaalf verschillende emmers");
    assert_eq!(toestand(&via_mint), toestand(&via_alias));
}

/// Het scenario uit de review van stap 7, zonder tussencontroles: Alice
/// krijgt een munt via `mint_to_ledger`, Mallory een munt in dezelfde emmer
/// (5,4) via `Ledger::mint`. Mallory besteedt eerst; daarna moet de besteding
/// van Alice slagen. Op stap 7 faalt alleen die laatste, met `LegeEmmer`.
#[test]
fn alice_na_mallory_in_dezelfde_emmer() {
    let mut l = Ledger::new(keten());
    let alice = sleutel(26);
    let mallory = sleutel(27);
    let munt_alice = munt(&alice, 4, 5, 1);
    let munt_mallory = munt(&mallory, 4, 5, 2);
    mint_to_ledger(&mut l, &munt_alice);
    l.mint(&munt_mallory);
    let rm = geldig(&l, &mallory, &munt_mallory, 1, uit(&l, 4, 2)).to_bytes();
    let mallory_uit = l.besteed(&rm).map(|_| ());
    let ra = geldig(&l, &alice, &munt_alice, 0, uit(&l, 4, 1)).to_bytes();
    let alice_uit = l.besteed(&ra).map(|_| ());
    assert_eq!(alice_uit, Ok(()), "besteding van Alice na Mallory");
    assert_eq!(mallory_uit, Ok(()), "besteding van Mallory");
}

/// De uitgang komt in emmer (epoch van de ledger, niveau), ook als C_uit met
/// een ander epoch is gemaakt.
#[test]
fn emmer_van_de_uitgang() {
    let mut l = Ledger::new(keten());
    let a = sleutel(20);
    let n = munt(&a, 4, 3, 1);
    mint_to_ledger(&mut l, &n);
    let c_uit = munt(&sleutel(21), 4, 7, 1).commitment(l.chain());
    let b = l.besteed(&geldig(&l, &a, &n, 0, c_uit).to_bytes()).unwrap();
    assert_eq!(l.epoch(), 0);
    assert_eq!(l.bucket_of(b.index_uit), Some((0, 4)));
    assert_eq!(l.buckets().get(&(0, 4)), Some(&1));
    assert_eq!(l.buckets().get(&(3, 4)), Some(&1));
    assert_eq!(l.buckets().get(&(7, 4)), None);
}

// ---- formaat --------------------------------------------------------------

fn een_munt() -> (Ledger, SigningKey, Note) {
    let mut l = Ledger::new(keten());
    let a = sleutel(22);
    let n = munt(&a, 4, 0, 1);
    mint_to_ledger(&mut l, &n);
    mint_to_ledger(&mut l, &munt(&a, 4, 0, 2));
    (l, a, n)
}

#[test]
fn formaat_lengte() {
    let (mut l, a, n) = een_munt();
    let goed = geldig(&l, &a, &n, 0, uit(&l, 4, 1)).to_bytes();
    assert_eq!(goed.len(), record_bytes(1));
    let mut lang = goed.clone();
    lang.push(0);
    weigert(&mut l, &lang, F::Formaat(Formaat::Lengte));
    weigert(&mut l, &goed[..goed.len() - 1], F::Formaat(Formaat::Lengte));
    weigert(&mut l, &[], F::Formaat(Formaat::Lengte));
    weigert(&mut l, &goed[..147], F::Formaat(Formaat::Lengte));
    assert!(l.besteed(&goed).is_ok());
}

#[test]
fn formaat_onbekende_versie() {
    let (mut l, a, n) = een_munt();
    let mut r = kaal(&l, &a, &n, 0, uit(&l, 4, 1));
    r.versie = 2;
    teken(l.chain(), &a, &mut r);
    weigert(&mut l, &r.to_bytes(), F::Formaat(Formaat::Versie));
}

#[test]
fn formaat_onbekend_schema() {
    let (mut l, a, n) = een_munt();
    let mut r = kaal(&l, &a, &n, 0, uit(&l, 4, 1));
    r.schema = 2;
    teken(l.chain(), &a, &mut r);
    weigert(&mut l, &r.to_bytes(), F::Formaat(Formaat::Schema));
}

/// Niveau 33, ook als er (via `mint`, dat niet controleert) een munt van
/// niveau 33 is opgenomen.
#[test]
fn formaat_niveau_33() {
    let mut l = Ledger::new(keten());
    let a = sleutel(23);
    let n = munt(&a, 33, 0, 1);
    mint_to_ledger(&mut l, &n);
    let r = geldig(&l, &a, &n, 0, uit(&l, 33, 1));
    weigert(&mut l, &r.to_bytes(), F::Formaat(Formaat::Niveau));
}

#[test]
fn formaat_diepte_65() {
    let (mut l, a, n) = een_munt();
    let mut r = kaal(&l, &a, &n, 0, uit(&l, 4, 1));
    r.bewijs = Proof { path: vec![[0u8; 64]; MAX_DIEPTE + 1] };
    teken(l.chain(), &a, &mut r);
    let b = r.to_bytes();
    assert_eq!(b.len(), record_bytes(65));
    weigert(&mut l, &b, F::Formaat(Formaat::Diepte));
}

// ---- toestand, behoud, robuustheid ----------------------------------------

/// Na elke geweigerde besteding zijn wortel, nullifiers, emmers, spent en
/// lengte gelijk aan ervoor; ook expliciet per veld, naast de volledige
/// Debug-vergelijking in `weigert`.
#[test]
fn geen_wijziging_bij_weigering() {
    let (mut l, a, n) = een_munt();
    let goed = geldig(&l, &a, &n, 0, uit(&l, 4, 1));
    let mut slecht_sig = goed.clone();
    slecht_sig.handtekening[0] ^= 1;
    let mut slecht_uit = goed.clone();
    slecht_uit.c_uit[0] ^= 1;
    let mut slecht_lid = goed.clone();
    slecht_lid.geheim_in[0] ^= 1;
    teken(l.chain(), &a, &mut slecht_lid);
    for r in [slecht_sig, slecht_uit, slecht_lid] {
        let nf = nullifier(l.chain(), 0, &n.commitment(l.chain()));
        let (w, len, em, sp) = (l.root(), l.acc().len(), l.buckets().clone(), l.spent().clone());
        let voor = toestand(&l);
        assert!(l.besteed(&r.to_bytes()).is_err());
        assert_eq!(l.root(), w);
        assert_eq!(l.acc().len(), len);
        assert_eq!(l.buckets(), &em);
        assert_eq!(l.spent(), &sp);
        assert!(!l.is_verbrand(&nf), "nullifier verbrand bij een weigering");
        assert_eq!(toestand(&l), voor);
    }
    assert!(l.besteed(&goed.to_bytes()).is_ok());
}

/// XOF-stroom uit een vaste seed, voor reproduceerbaar willekeurig werk.
struct Stroom(blake3::OutputReader);
impl Stroom {
    fn new(seed: &[u8]) -> Self { let mut h = blake3::Hasher::new(); h.update(seed); Stroom(h.finalize_xof()) }
    fn u64(&mut self) -> u64 { let mut b = [0u8; 8]; self.0.fill(&mut b); u64::from_le_bytes(b) }
    fn tot(&mut self, n: u64) -> u64 { self.u64() % n }
    fn bytes(&mut self, n: usize) -> Vec<u8> { let mut b = vec![0u8; n]; self.0.fill(&mut b); b }
}

/// Behoud: na een reeks geldige bestedingen is per niveau het aantal
/// besteedbare munten gelijk aan het aantal uitgegeven munten.
#[test]
fn behoud_per_niveau() {
    let mut l = Ledger::new(keten());
    let mut s = Stroom::new(b"gravity-ledger/behoud/v1");
    let niveaus = [0u8, 5, 17, 32];
    let mut uitgegeven = [0usize; 33];
    // (sleutel, munt, positie) van nog niet bestede munten.
    let mut open: Vec<(u8, Note, usize)> = Vec::new();
    for (j, &lv) in niveaus.iter().enumerate() {
        for k in 0..6u8 {
            let sk = 30 + k;
            let n = munt(&sleutel(sk), lv, j as u64, k);
            let i = mint_to_ledger(&mut l, &n);
            uitgegeven[lv as usize] += 1;
            open.push((sk, n, i));
        }
    }
    for stap in 0..120u32 {
        let pos = s.tot(open.len() as u64) as usize;
        let (sk, n, i) = open.swap_remove(pos);
        let nieuw_sk = 40 + (stap % 7) as u8;
        let mut nieuw = munt(&sleutel(nieuw_sk), n.level, l.epoch(), 0);
        nieuw.secret.0[4..8].copy_from_slice(&stap.to_le_bytes());
        let r = geldig(&l, &sleutel(sk), &n, i, nieuw.commitment(l.chain()));
        let b = l.besteed(&r.to_bytes()).unwrap_or_else(|e| panic!("stap {stap}: geldige besteding geweigerd: {e:?}"));
        open.push((nieuw_sk, nieuw, b.index_uit));
    }
    for lv in 0..33u8 {
        let besteedbaar: usize = l.buckets().keys().filter(|(_, v)| *v == lv)
            .map(|&(e, v)| live(l.buckets(), l.spent(), e, v).unwrap()).sum();
        assert_eq!(besteedbaar, uitgegeven[lv as usize], "niveau {lv}");
    }
}

/// Minstens 10.000 willekeurige, afgekapte en verminkte records uit een vaste
/// seed: nooit een paniek, nooit een aanvaarding, nooit een wijziging.
#[test]
fn robuustheid_tienduizend_records() {
    let (mut l, a, n) = een_munt();
    let goed = geldig(&l, &a, &n, 0, uit(&l, 4, 1)).to_bytes();
    let mut s = Stroom::new(b"gravity-ledger/robuustheid/v1");
    let voor = toestand(&l);
    let mut aantal = 0usize;
    let mut probeer = |l: &mut Ledger, r: &[u8]| {
        assert!(l.besteed(r).is_err(), "record van {} byte aanvaard", r.len());
        aantal += 1;
    };
    // 1. Elke afgekapte vorm van een geldig record.
    for k in 0..goed.len() { probeer(&mut l, &goed[..k]); }
    for _ in 0..2_500 {
        // 2. Volledig willekeurig, 0..=700 byte.
        let len = s.tot(701) as usize;
        let r = s.bytes(len);
        probeer(&mut l, &r);
        // 3. Geldige kop, willekeurige rest, juiste lengte.
        let d = s.tot(MAX_DIEPTE as u64 + 1) as usize;
        let mut r = s.bytes(record_bytes(d));
        r[0] = VERSIE; r[1] = Ed25519::SCHEMA_ID; r[10] = (s.tot(33)) as u8; r[147] = d as u8;
        probeer(&mut l, &r);
        // 4. Geldig record met 1 tot 4 verminkte bytes.
        let mut r = goed.clone();
        for _ in 0..=s.tot(4) {
            let p = s.tot(r.len() as u64) as usize;
            r[p] ^= (s.tot(255) + 1) as u8;
        }
        if r != goed { probeer(&mut l, &r); }
        // 5. Geldig record, afgekapt of verlengd met willekeurige bytes.
        let mut r = goed.clone();
        if s.tot(2) == 0 { r.truncate(s.tot(goed.len() as u64) as usize); } else { let x = s.tot(200) as usize + 1; r.extend(s.bytes(x)); }
        probeer(&mut l, &r);
    }
    assert!(aantal >= 10_000, "maar {aantal} records geprobeerd");
    assert_eq!(toestand(&l), voor, "ledger gewijzigd door geweigerde records");
    assert!(l.besteed(&goed).is_ok(), "het geldige record hoort daarna nog te slagen");
}
