//! Blok 2 — meetkader. Meet, bewijst niets.
//!
//! Staat in de repo (niet in /tmp) zodat meetwerk niet meer verloren gaat.
//! Draait alleen expliciet: `cargo test --release -- --ignored --nocapture`.
//! Uitkomsten van drie runs: `docs/metingen.md`.
//!
//! Het doel: hoe groot en hoe traag zijn een lidmaatschapsbewijs, een
//! zichtbare besteding en een post-kwantumhandtekening bij 1.000 tot 100.000
//! bladen?

#[cfg(test)]
mod tests {
    use crate::{conservation, hash, tree};
    use std::time::Instant;

    fn leaf_of(i: usize) -> hash::Digest {
        hash::hash(&(i as u64).to_le_bytes())
    }

    #[test]
    #[ignore = "meting — draait alleen met --ignored"]
    fn meet_ladder_en_bewijzen() {
        println!("verwijzing: digest = {} byte (BLAKE3-XOF, één primitief)", hash::DIGEST_BYTES);

        // Begrensd: 1M kost ~10 s per root op één kern en past dan niet in één run.
        for &n in &[1_000usize, 10_000, 100_000] {
            // Bouw de boom from scratch en meet de twee getallen die er toe
            // doen: wat kost één mint (amortiseerd), en wat kost het opvragen
            // van de wortel? Die zijn nu gescheiden — voorheen niet.
            let mut a = tree::Accumulator::new();
            let t = Instant::now();
            for i in 0..n { a.push(leaf_of(i)); }
            let mint_us = t.elapsed().as_secs_f64() * 1e6 / n as f64;

            let t2 = Instant::now();
            for _ in 0..1000 { std::hint::black_box(a.root()); }
            let root_ns = t2.elapsed().as_secs_f64() * 1e9 / 1000.0;
            let root = a.root();

            // Bewijzen: verificatie is O(log n), opbouw is dat níet.
            const K: usize = 256;
            let pre: Vec<(usize, tree::Proof)> = (0..K)
                .map(|s| { let idx = (s * 977) % n; (idx, a.proof(idx).expect("bewijs moet bestaan")) })
                .collect();
            let diepte = pre[0].1.depth();
            let bytes = pre[0].1.size_bytes();

            let t1 = Instant::now();
            for _ in 0..40 {
                for (idx, p) in &pre { std::hint::black_box(p.verify(*idx, &a.leaves[*idx], &root)); }
            }
            let verif_us = t1.elapsed().as_secs_f64() * 1e6 / (40 * K) as f64;

            let t2 = Instant::now();
            for s in 0..K { std::hint::black_box(a.proof((s * 977) % n)); }
            let bouw_us = t2.elapsed().as_secs_f64() * 1e6 / K as f64;

            let ok = pre.iter().filter(|(idx, p)| p.verify(*idx, &a.leaves[*idx], &root)).count();

            // Wat kost de concurrency-laag? Let op: de huidige wortel keert
            // meteen terug via de eerste toets, dus die meten is bedrog. Deze
            // twee metingen doen het echte werk: een verouderde maar geldige
            // wortel (scan door de log tot treffer) en een wortel buiten het
            // venster (volledige scan, dan afwijzing — de ergste casus).
            let mut b = tree::Accumulator::new();
            for i in 0..(n - 3) { b.push(leaf_of(i)); }
            let root_oud = b.root();
            for i in (n - 3)..n { b.push(leaf_of(i)); }
            assert_eq!(b.root(), root, "vergelijkingboom moet identiek zijn");
            let onbekend = leaf_of(usize::MAX);

            let t3 = Instant::now();
            for _ in 0..K { std::hint::black_box(a.accepts_root(&root_oud)); }
            let venster_us = t3.elapsed().as_secs_f64() * 1e6 / K as f64;

            let t5 = Instant::now();
            for _ in 0..K { std::hint::black_box(a.accepts_root(&onbekend)); }
            let afwijzing_us = t5.elapsed().as_secs_f64() * 1e6 / K as f64;

            let t4 = Instant::now();
            for (idx, p) in &pre { std::hint::black_box(a.verify_at(p, *idx, &a.leaves[*idx], &root_oud)); }
            let met_venster_us = t4.elapsed().as_secs_f64() * 1e6 / K as f64;

            println!(
                "n={:>7} | MINT {:>6.2} µs | WORTEL {:>6.0} ns (O(1)) | bewijs {} byte (diepte {}) | VERIFIEREN {:>5.2} µs | +VENSTER {:>5.2} µs | venster scan {:>6.3} µs | afwijzing {:>6.3} µs | BEWIJS MAKEN {:>7.2} µs | correct {}/{}",
                n, mint_us, root_ns, bytes, diepte, verif_us, met_venster_us, venster_us, afwijzing_us, bouw_us, ok, K
            );
        }

        // Waardebehoud: 33 niveaus, één controle — de kern van het ontwerp.
        let mut s = conservation::Shape::default();
        s.ins[20] = 1_000; s.splits[20] = 500; s.outs[20] = 500; s.outs[19] = 1_000;
        assert_eq!(s.check(), Ok(()));
        let t = Instant::now();
        for _ in 0..2_000_000 { std::hint::black_box(s.check()).unwrap(); }
        println!("behoud-controle: {:.0} ns per controle over 33 niveaus",
                 t.elapsed().as_secs_f64() * 1e9 / 2_000_000.0);
    }

    /// De PQ-vloer: wat kost één post-kwantum handtekening, in bytes en tijd?
    /// Dit is de ondergrens van elke transactie — groter dan het hele
    /// lidmaatschapsbewijs hierboven. Meten met dilithium-rs (ML-DSA, FIPS 204).
    #[test]
    #[ignore = "meting — draait alleen met --ignored"]
    fn meet_pq_vloer() {
        use dilithium::{DilithiumKeyPair, DilithiumMode, ML_DSA_44, ML_DSA_65, ML_DSA_87};

        let boodschap = b"gravity-ledger: besteding van een noot";
        println!("ter vergelijking: Merkle-lidmaatschapsbewijs bij 100k bladeren = 1088 byte (64 byte per niveau, stap 6)");

        for (naam, mode) in [("ML-DSA-44", ML_DSA_44), ("ML-DSA-65", ML_DSA_65), ("ML-DSA-87", ML_DSA_87)] {
            let mode: DilithiumMode = mode;
            let mut seed = [0u8; 32];
            seed[0] = naam.len() as u8;
            let kp = DilithiumKeyPair::generate_deterministic(mode, &seed);
            let pk = kp.public_key().to_vec();

            let sig = kp.sign(boodschap, b"").expect("signeren moet lukken");
            let sig_bytes = sig.as_bytes().len();

            // Correctheid: geldige signatuur moet doorlopen, vervalsing niet.
            assert!(DilithiumKeyPair::verify(&pk, &sig, boodschap, b"", mode), "geldig bewijs geweigerd");
            assert!(!DilithiumKeyPair::verify(&pk, &sig, b"andere boodschap", b"", mode), "vervalste boodschap geaccepteerd");

            const N: usize = 30;
            let t = Instant::now();
            for _ in 0..N { std::hint::black_box(kp.sign(boodschap, b"").unwrap()); }
            let sign_us = t.elapsed().as_secs_f64() * 1e6 / N as f64;

            let t2 = Instant::now();
            for _ in 0..N { std::hint::black_box(DilithiumKeyPair::verify(&pk, &sig, boodschap, b"", mode)); }
            let verif_us = t2.elapsed().as_secs_f64() * 1e6 / N as f64;

            println!(
                "{:<9} | publieke sleutel {:>5} byte | signatuur {:>5} byte | signer {:>7.0} µs | verifiëren {:>6.0} µs",
                naam, pk.len(), sig_bytes, sign_us, verif_us
            );
        }
    }

    /// FALCON / FN-DSA (FIPS 206). `falcon-rs` geeft de handtekening in het
    /// constant-time-formaat (809 / 1.577 byte; padded is 666 / 1.280 byte).
    /// Dezelfde meting als hierboven voor ML-DSA, zodat de twee op dezelfde
    /// voet vergeleken worden.
    #[test]
    #[ignore = "meting — draait alleen met --ignored"]
    fn meet_falcon_vloer() {
        use falcon::safe_api::{DomainSeparation, FnDsaKeyPair, FnDsaSignature};

        let boodschap = b"gravity-ledger: besteding van een noot";
        println!("ter vergelijking: ML-DSA-44 = 2420 byte | Merkle-bewijs bij 100k = 1088 byte");

        // logn is log2(n): 9 -> Falcon-512 (FN-DSA level 1), 10 -> Falcon-1024.
        for logn in [9u32, 10] {
            let seed = [logn as u8; 32];
            let kp = FnDsaKeyPair::generate_deterministic(&seed, logn).expect("sleutelpaar genereren");
            let pk = kp.public_key().to_vec();

            let sig = kp.sign(boodschap, &DomainSeparation::None).expect("signeren");
            let sig_bytes = sig.to_bytes().len();

            // Correctheid: geldige signatuur moet aanvaard worden, vervalsing niet.
            assert!(FnDsaSignature::verify(sig.to_bytes(), &pk, boodschap, &DomainSeparation::None).is_ok(),
                    "geldige signatuur verworpen");
            assert!(FnDsaSignature::verify(sig.to_bytes(), &pk, b"andere boodschap", &DomainSeparation::None).is_err(),
                    "vervalste boodschap aanvaard");

            const N: usize = 30;
            let t = Instant::now();
            for _ in 0..N { std::hint::black_box(kp.sign(boodschap, &DomainSeparation::None).unwrap()); }
            let sign_us = t.elapsed().as_secs_f64() * 1e6 / N as f64;

            let t2 = Instant::now();
            for _ in 0..N {
                std::hint::black_box(FnDsaSignature::verify(sig.to_bytes(), &pk, boodschap, &DomainSeparation::None).is_ok());
            }
            let verif_us = t2.elapsed().as_secs_f64() * 1e6 / N as f64;

            println!(
                "{:<12} | publieke sleutel {:>5} byte | signatuur {:>4} byte | signer {:>7.0} µs | verifiëren {:>6.0} µs",
                kp.variant_name(), pk.len(), sig_bytes, sign_us, verif_us
            );
        }
    }

    /// Wat kost één zichtbare besteding (stap 7), en hoe groot is het record?
    #[test]
    #[ignore = "meting — draait alleen met --ignored"]
    fn meet_besteding() {
        use crate::besteding::{bericht, nullifier, record_bytes, Record, VERSIE};
        use crate::handtekening::{Ed25519, Handtekening};
        use crate::note::{owner_hash, ChainId, Note, Secret};
        use crate::spend;
        use crate::Ledger;
        use ed25519_dalek::{Signer, SigningKey};

        let chain = ChainId([0x42; 32]);
        let sk = SigningKey::from_bytes(&[7; 32]);
        let pk = sk.verifying_key().to_bytes();
        let owner = owner_hash(Ed25519::SCHEMA, &pk);
        let noot = |i: usize| -> Note {
            let mut sec = [0u8; 32];
            sec[..8].copy_from_slice(&(i as u64).to_le_bytes());
            Note { level: 7, epoch: 0, owner, secret: Secret(sec) }
        };

        for &n in &[1_000usize, 10_000, 100_000] {
            let mut l = Ledger::new(chain.clone());
            for i in 0..n { spend::mint_to_ledger(&mut l, &noot(i)); }
            // Alle records wijzen naar de wortel van vóór de eerste besteding;
            // elke besteding voegt een wortel toe, dus K blijft binnen het venster (64).
            const K: usize = 60;
            let records: Vec<Vec<u8>> = (0..K).map(|s| {
                let i = (s * 977) % n;
                let c_in = noot(i).commitment(&chain);
                let c_uit = noot(n + i).commitment(&chain);
                let nf = nullifier(&chain, i as u64, &c_in);
                let m = bericht(&chain, VERSIE, Ed25519::SCHEMA_ID, &nf, 7, &c_uit);
                Record { versie: VERSIE, schema: Ed25519::SCHEMA_ID, index_in: i as u64, niveau: 7, epoch_in: 0,
                         geheim_in: noot(i).secret.0, pk, wortel: l.root(), bewijs: l.acc.proof(i).unwrap(),
                         c_uit, handtekening: sk.sign(&m).to_bytes() }.to_bytes()
            }).collect();
            let diepte = l.acc.proof(0).unwrap().depth();
            assert_eq!(records[0].len(), record_bytes(diepte));
            let t = Instant::now();
            for r in &records { l.besteed(r).expect("geldige besteding geweigerd"); }
            let us = t.elapsed().as_secs_f64() * 1e6 / K as f64;
            println!("n={:>7} | record {} byte (diepte {}) | +~200 overhead: {} van 4096 | besteed {:.1} µs ({} records)",
                     n, records[0].len(), diepte, records[0].len() + 200, us, K);
        }
    }
}
