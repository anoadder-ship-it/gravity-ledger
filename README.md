# gravity-ledger

Hash-only lidmaatschap en waardenbehoud — een Rust-basislaag die verifieert dat
notan in een accumulator horen, en dat waarde behouden blijft, zonder getallen
te onthullen.

**Wat het is:** een bewezen, gemeten kern — hashlaag, notan-commitments,
incrementele Merkle-accumulator en waardenbehoud-telling.

**Wat het níet is (nog niet):** een anonieme bestedingslaag. Een bewijs dat je
het geheim bezit bij *één van de N* notan zonder te zeggen welke, bestaat als
post-kwantum primitief niet in productie. Zie `STATUS.md` sectie "open".

## Architectuur

| laag | bestand | wat het doet |
|---|---|---|
| hash | `src/hash.rs` | BLAKE3 extended naar 64 byte (XOF), met domeinscheiding per context |
| notan | `src/note.rs` | commitment, nullifier en audit-tag per noot; 33 denominaties (2^0…2^32) |
| accumulator | `src/tree.rs` | append-only Merkle-boom, incrementeel bijgehouden, index-gebonden bladeren |
| behoud | `src/conservation.rs` | waardenbehoud als telling over 33 niveaus — geen reeks-bewijzen nodig |
| keten | `src/lib.rs` | mint / nullify / root; dubbeluitgave-afdoening |
| meting | `src/bench.rs` | de getallen hieronder, als `#[ignore]`-test in de repo |

Ontwerpkeuzes en hun onderbouwing staan in `STATUS.md`.

## Gemeten (release-build, één kern)

| anonimiteitsset | mint | wortel | bewijs | verifiëren | bewijs maken |
|---|---|---|---|---|---|
| 1.000 | 2,6 µs | 2 ns | 650 byte | 2,6 µs | 0,04 µs |
| 10.000 | 3,3 µs | 2 ns | 910 byte | 3,5 µs | 0,06 µs |
| 100.000 | 4,1 µs | 2 ns | 1.105 byte | 4,1 µs | 0,17 µs |

Waardebehoud-control: **19 ns** over 33 niveaus.

Post-kwantum ondergrens (`dilithium-rs` 0.4.1, ML-DSA / FIPS 204):

| | publieke sleutel | signatuur | signeren | verifiëren |
|---|---|---|---|---|
| ML-DSA-44 | 1.312 byte | 2.420 byte | 275 µs | 69 µs |
| ML-DSA-65 | 1.952 byte | 3.309 byte | 449 µs | 67 µs |
| ML-DSA-87 | 2.592 byte | 4.627 byte | 264 µs | 79 µs |

**Gevolg voor het ontwerp:** één PQ-handtekening is 2–4× zo groot als het hele
lidmaatschapsbewijs. De handtekening, niet de boom, bepaalt de ondergrens per
transactie — dus: **één handtekening per transactie**, niet per noot.

## Bouwen en testen

```bash
cargo test                                    # 46 tests
cargo test -- --ignored --nocapture           # de metingen hierboven
```

Geen netwerk nodig: alle afhankelijkheden staan in de lokale cache.

## Afhankelijkheden en auditstatus

| crate | waarvoor | status |
|---|---|---|
| `blake3` 1.8.6 | enige primitief in productiecode | officiële implementatie, NCC Group-geaudit (2020) |
| `sha2` 0.10.9 | uitsluitend dev: onafhankelijke KAT-vectoren | RustCrypto |
| `dilithium-rs` 0.4.1 | uitsluitend dev: PQ-meting | twee auditrondes (2026-07, 2026-08); alle twaalf bevindingen, incl. één HIGH, staan als fixed |

**Waarom geen `ml-dsa` (RustCrypto):** die crate draagt de expliciete melding
*"never been independently audited — USE AT YOUR OWN RISK"* en had
CVE-2026-24850 (verificatie aanvaardde handtekeningen met dubbele
hint-indexen). Dit project gebruikt die crate niet; de metingen draaien op de
tweemaal geauditeerde `dilithium-rs`.

## Open

1. Anonieme bestedingslaag — vereist een "één van de N"-bewijs (post-kwantum:
   research, geen implementatie).
2. Solana-integratie — nog geen regel code; vergelijk eerst met Light Protocol
   (concurrent Merkle trees, ZK Compression) en Token-2022 Confidential
   Transfers, die beide al bestaan.
3. Hybride handtekening (ML-DSA + Ed25519) als aanbevolen verdediging tegen
   implementatiefouten in één van beide.

## License

Apache-2.0
