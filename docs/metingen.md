# Metingen

Dit zijn **drie runs op één machine**. Tijden wisselen per run en per machine;
lees ze als orde van grootte, niet als vaste waarde. Maten in bytes zijn vast:
ze volgen uit het formaat en waren in alle drie de runs gelijk.

## Opzet

| | |
|---|---|
| datum | 2026-11-13 |
| machine | aarch64 (ARM), 20 kernen: 10 × Cortex-X925 (tot 3,9 GHz) en 10 × Cortex-A725 (tot 2,8 GHz) |
| Rust | rustc 1.97.1 (stable), release-build |
| commando | `cargo test --release -- --ignored --nocapture` (hier met `--offline --test-threads=1`) |

De threads zijn niet aan een kern vastgezet; de planner kan een meting op een
snelle of een zuinige kern laten lopen. Dat verklaart een deel van de spreiding.

De metingen staan in `src/bench.rs`:

| meting | wat | herhalingen per getal |
|---|---|---|
| `meet_ladder_en_bewijzen` | accumulator bij 1.000, 10.000 en 100.000 bladen; waardebehoud | mint: gemiddeld over n opnames; wortel: 1.000; verifiëren: 40 × 256 bewijzen; bewijs maken, venster en afwijzing: 256; behoud: 2.000.000 |
| `meet_besteding` | één zichtbare besteding (`Ledger::besteed`) bij 1.000, 10.000 en 100.000 bladen | 60 records |
| `meet_pq_vloer` | ML-DSA-44, -65 en -87 (`dilithium-rs`, dev-afhankelijkheid) | 30 keer tekenen en 30 keer verifiëren |
| `meet_falcon_vloer` | FN-DSA-512 en -1024 (`falcon-rs`, dev-afhankelijkheid) | 30 keer tekenen en 30 keer verifiëren |

## Tijden: laagste en hoogste waarde van drie runs

### Accumulator (`meet_ladder_en_bewijzen`)

| bladen | mint | wortel | verifiëren | verifiëren met oudere wortel in het venster | bewijs maken | vensterscan | afwijzing buiten het venster |
|---|---|---|---|---|---|---|---|
| 1.000 | 1,99–3,78 µs | 2–3 ns | 1,98–3,13 µs | 1,99–2,05 µs | 0,03–0,29 µs | 76–78 ns | 79–82 ns |
| 10.000 | 2,63–2,70 µs | 2 ns | 2,72–2,77 µs | 2,84–2,95 µs | 0,04–0,05 µs | 82–89 ns | 80–84 ns |
| 100.000 | 3,35–3,40 µs | 2 ns | 3,28–3,37 µs | 3,62–4,61 µs | 0,14–0,55 µs | 83–96 ns | 79–81 ns |

Waardebehoud (`Shape::check` over 33 niveaus): 20 ns in alle drie de runs.

### Besteding (`meet_besteding`)

| bladen | tijd per besteding |
|---|---|
| 1.000 | 43,9–109,8 µs |
| 10.000 | 43,9–45,6 µs |
| 100.000 | 46,1–47,2 µs |

### Post-kwantumhandtekeningen (`meet_pq_vloer`, `meet_falcon_vloer`)

| schema | tekenen | verifiëren |
|---|---|---|
| FN-DSA-512 | 267–485 µs | 26–51 µs |
| FN-DSA-1024 | 540–549 µs | 50–52 µs |
| ML-DSA-44 | 218–279 µs | 69 µs |
| ML-DSA-65 | 323–397 µs | 113 µs |
| ML-DSA-87 | 221–342 µs | 82–84 µs |

De tekentijd van ML-DSA wisselt ook door het algoritme zelf
(afwijzingsbemonstering): een handtekening kan meer pogingen nodig hebben. Daarom
is ML-DSA-87 in sommige runs sneller dan ML-DSA-65.

## Maten (vast)

| bladen | diepte | bewijs | record (276 + 64 × diepte) |
|---|---|---|---|
| 1.000 | 10 | 640 byte | 916 byte |
| 10.000 | 14 | 896 byte | 1.172 byte |
| 100.000 | 17 | 1.088 byte | 1.364 byte |

Een digest is 64 byte (BLAKE3-XOF). `meet_besteding` drukt ook record + 200 byte
af (1.116 / 1.372 / 1.564 byte); die 200 byte overhead is een aanname, geen
meting.

| schema | publieke sleutel | handtekening zoals `falcon-rs` / `dilithium-rs` die teruggeeft |
|---|---|---|
| FN-DSA-512 | 897 byte | 809 byte |
| FN-DSA-1024 | 1.793 byte | 1.577 byte |
| ML-DSA-44 | 1.312 byte | 2.420 byte |
| ML-DSA-65 | 1.952 byte | 3.309 byte |
| ML-DSA-87 | 2.592 byte | 4.627 byte |

## Formaat van de Falcon-handtekening

`falcon-rs` 0.3.1 tekent in het **constant-time-formaat** (CT): `safe_api.rs`
roept `falcon_sign_dyn_finish` aan met `FALCON_SIG_CT`, de kopbyte is
`0x50 + logn`, en de lengte is `falcon_sig_ct_size(logn)` = 809 byte
(FN-DSA-512) en 1.577 byte (FN-DSA-1024). Dat zijn ook de lengtes die de
meting afdrukt. De documentatie van `falcon-rs` zelf noemt bij het CT-formaat
666 en 1.280 byte; volgens de eigen functie `falcon_sig_padded_size` van het
crate zijn dat de groottes van het *padded*-formaat. Het IETF-concept
draft-ietf-cose-falcon-04 (tabel 1) noemt eveneens 666 en 1.280 byte.

Budget van een zichtbare besteding met FN-DSA-512 in plaats van Ed25519
(record − 32 − 64 + 897 + handtekening), berekend, tegen een limiet van 4.096
byte:

| bladen | met 809 byte (CT) | met 666 byte (padded) |
|---|---|---|
| 100.000 | 2.974 byte (1.122 over) | 2.831 byte (1.265 over) |
| 2^32 | 3.934 byte (162 over) | 3.791 byte (305 over) |

De overhead van een transactie bovenop het record is niet gemeten. Of het
budget bij 2^32 bladen past, is daarom met deze cijfers niet vast te stellen;
met 666 byte is het wel minder krap.
