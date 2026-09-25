# gravity-ledger

Hash-only lidmaatschap en waardenbehoud — een Rust-basislaag die verifieert dat
notan in een accumulator horen, en dat waarde behouden blijft, zonder getallen
te onthullen.

**Wat het is:** een bewezen, gemeten kern — hashlaag, notan-commitments,
incrementele Merkle-accumulator met acceptatievenster voor gelijktijdige
schrijvers, en waardenbehoud-telling.

**Wat het níet is (nog niet):** een anonieme bestedingslaag. Een bewijs dat je
het geheim bezit bij *één van de N* notan zonder te zeggen welke, bestaat als
post-kwantum primitief niet in productie. Zie `STATUS.md` sectie 6.

## Architectuur

| laag | bestand | wat het doet |
|---|---|---|
| hash | `src/hash.rs` | BLAKE3 extended naar 64 byte (XOF), met domeinscheiding per context |
| notan | `src/note.rs` | commitment, nullifier en audit-tag per noot; 33 denominaties (2^0…2^32) |
| accumulator | `src/tree.rs` | append-only Merkle-boom, incrementeel, index-gebonden bladeren, wortel-venster |
| behoud | `src/conservation.rs` | waardenbehoud als telling over 33 niveaus — geen reeks-bewijzen nodig |
| keten | `src/lib.rs` | mint / nullify / root; dubbeluitgave-afdoening |
| meting | `src/bench.rs` | alle getallen hieronder, als `#[ignore]`-test in de repo |

## Gemeten (release-build, één kern)

| anonimiteitsset | mint | wortel | bewijs | verifiëren | +venster |
|---|---|---|---|---|---|
| 1.000 | 2,6 µs | 2 ns | 650 byte | 2,7 µs | 2,6 µs |
| 10.000 | 3,3 µs | 2 ns | 910 byte | 3,5 µs | 3,5 µs |
| 100.000 | 4,1 µs | 2 ns | 1.105 byte | 4,2 µs | 4,3 µs |

Waardebehoud: **19 ns**. Vensterscan: **73–85 ns**.

Post-kwantum handtekeningen (zelfde harness):

| | publieke sleutel | signatuur | signeren | verifiëren |
|---|---|---|---|---|
| **FN-DSA-512** (Falcon) | 897 byte | **809 byte** | 480 µs | 51 µs |
| FN-DSA-1024 (Falcon) | 1.793 byte | 1.577 byte | 526 µs | 49 µs |
| ML-DSA-44 | 1.312 byte | 2.420 byte | 148 µs | 29 µs |
| ML-DSA-87 | 2.592 byte | 4.627 byte | 258 µs | 79 µs |

**Gevolg voor het ontwerp:** Solana noemt FALCON als richting (Anza, SIMD-0461)
en dat is meetbaar de juiste keuze — drie keer kleiner dan ML-DSA-44. Budget per
besteding: 809 + 1.105 = **1.914 byte** tegen een limiet van 4.096 (SIMD-0296,
live sinds 15 september 2026). De handtekening bepaalt de ondergrens, dus:
**één handtekening per transactie**, niet per noot.

## Bouwen en testen

```bash
cargo test                                # 50 tests
cargo test -- --ignored --nocapture       # de metingen hierboven
```

## Afhankelijkheden en auditstatus

| crate | waarvoor | status |
|---|---|---|
| `blake3` 1.8.6 | enige primitief in productiecode | officiële implementatie, NCC Group-geaudit (2020) |
| `sha2` 0.10.9 | dev-only: onafhankelijke KAT-vectoren | RustCrypto |
| `dilithium-rs` 0.4.1 | dev-only: PQ-meting | twee auditrondes, alle twaalf bevindingen fixed |
| `falcon-rs` 0.3.1 | dev-only: PQ-meting | niet geauditeerd — meten ja, tekenen nee |

Vrachtaudit draait: `cargo audit` tegen 1.269 RustSec-advisories, geen
kwetsbaarheden in de 25+ afhankelijkheden. De scanner is daarvóór gevalideerd op
een opzettelijk kwetsbaar project (vond RUSTSEC-2022-0093 en -2024-0344).

## Waarom geen Light Protocol, geen SNARK

Light Protocol (ZK Compression) is volwassen — drie audits, productiegebruik —
maar hun boom hashet Poseidon over **BN254** (`ark-bn254`) en het protocol wordt
geleverd met gnark-toetsen. Dat betekent ~32-byte knooppunten (onder BHT ruwweg
85 bit botsingsvastheid) en een SNARK met setup. Onze bindende commitment is
juist dát wat niet verwaterd mag worden: een botsing in de boom is nieuw geld.
Wat we wél overnemen is hun audit-lat en hun inzicht dat één boom veel
gelijktijdige schrijvers moet kunnen hebben.

## Open

1. Anonieme bestedingslaag — vereist een "één van de N"-bewijs (research, geen
   implementatie).
2. Solana-integratie — nog geen regel code.
3. Hybride signatuur FN-DSA + Ed25519, met gepinde versies en vrachtaudit in CI.

## License

Apache-2.0
