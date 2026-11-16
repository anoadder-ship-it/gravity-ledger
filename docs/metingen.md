# Metingen

Dit zijn **drie runs op één machine**. Tijden wisselen per run en per machine;
lees ze als orde van grootte, niet als vaste waarde. Maten in bytes zijn vast:
ze volgen uit het formaat en waren in alle drie de runs gelijk.

## Opzet

| | |
|---|---|
| datum | 2026-11-16 |
| machine | aarch64 (ARM), 20 kernen: 10 × Cortex-X925 (tot 3,9 GHz) en 10 × Cortex-A725 (tot 2,8 GHz) |
| Rust | rustc 1.97.1 (stable), release-build |
| commando | `cargo test --release -- --ignored --nocapture` (hier met `--offline --test-threads=1`) |

De threads zijn niet aan een kern vastgezet; de planner kan een meting op een
snelle of een zuinige kern laten lopen. Dat kan een deel van de spreiding
verklaren; onderzocht is dat niet.

Alle tijden in dit document komen uit dezelfde drie runs: de tweede meetronde
van 2026-11-16, gedraaid nadat het aantal herhalingen voor "bewijs maken" was
verhoogd (zie hieronder). De eerste ronde van die dag, met 256 aanroepen voor
"bewijs maken", staat in commit `c4565cb`. De vorige versie van dit
document (metingen van 2026-11-13) is de versie in commit `54ed7b8`
(`git show 54ed7b8:docs/metingen.md`). Ook
de kolommen die toen goed gemeten waren, zijn opnieuw gemeten en niet
overgenomen: de getimede lussen tellen sinds 2026-11-16 hun uitkomsten, dus
de oude getallen komen van andere code, en zo staat er één meetsessie in het
document in plaats van een mengsel van datums.

Elke getimede controle telt hoe vaak de uitkomst klopt, en de meting eist die
telling na de lus: alle geslaagd, of alle geweigerd waar weigeren de bedoeling
is. Een meting die een ander pad meet dan bedoeld, wordt rood en levert geen
getal op. Alle drie de runs waren groen.

De metingen staan in `src/bench.rs`:

| meting | wat | herhalingen per getal |
|---|---|---|
| `meet_ladder_en_bewijzen` | accumulator bij 1.000, 10.000 en 100.000 bladen; waardebehoud | mint: gemiddeld over n opnames; wortel: 1.000; verifiëren, in het venster en buiten het venster: elk 40 × 256 bewijzen; bewijs maken: 300.000 × 256 = 76.800.000 aanroepen (een lus van 2,34–6,01 s); vensterscan en afwijzing: 256; behoud: 2.000.000 |
| `meet_besteding` | één zichtbare besteding (`Ledger::besteed`) bij 1.000, 10.000 en 100.000 bladen | 60 records |
| `meet_pq_vloer` | ML-DSA-44, -65 en -87 (`dilithium-rs`, dev-afhankelijkheid) | 30 keer tekenen en 30 keer verifiëren |
| `meet_falcon_vloer` | FN-DSA-512 en -1024 (`falcon-rs`, dev-afhankelijkheid) | 30 keer tekenen en 30 keer verifiëren |

## Tijden: laagste en hoogste waarde van drie runs

### Accumulator (`meet_ladder_en_bewijzen`)

| bladen | mint | wortel | verifiëren | in het venster | buiten het venster (geweigerd) | bewijs maken | vensterscan | afwijzing buiten het venster |
|---|---|---|---|---|---|---|---|---|
| 1.000 | 1,99–2,01 µs | 3 ns | 1,99–2,01 µs | 2,09–2,12 µs | 2,09–2,10 µs | 30–31 ns | 80–90 ns | 81–93 ns |
| 10.000 | 2,64–2,66 µs | 3–4 ns | 2,74–2,75 µs | 2,81–2,84 µs | 2,82–2,84 µs | 41 ns | 84–85 ns | 79–81 ns |
| 100.000 | 3,33–3,45 µs | 3–4 ns | 3,30–3,32 µs | 3,36–3,45 µs | 3,38–3,42 µs | 78 ns | 81–91 ns | 79–81 ns |

Wat de kolommen meten:

- **verifiëren**: `Proof::verify` tegen de huidige wortel; alle 10.240
  controles slagen.
- **in het venster**: `Accumulator::verify_at` met bewijzen die zijn gemaakt
  toen de boom 3 bladen kleiner was, tegen de wortel van dat moment. Die wortel
  zit nog in het venster; alle 10.240 controles slagen. Dit is de vouwing plus
  `accepts_root`.
- **buiten het venster (geweigerd)**: hetzelfde, met bewijs en wortel van 64
  bladen terug. Het bewijs is waar tegen die wortel (dat wordt vooraf geëist),
  maar de wortel is uit het venster gevallen; alle 10.240 controles weigeren.
- **bewijs maken**: `Accumulator::proof` voor dezelfde 256 posities, 300.000
  keer achter elkaar; alle 76.800.000 aanroepen geven een bewijs. Tot de
  tweede ronde van 2026-11-16 waren het 256 aanroepen: een lus van ongeveer
  8 tot 36 µs (256 × 0,03–0,14 µs, de uitkomsten van de eerste ronde).
- **vensterscan**: alleen `accepts_root` voor de wortel van 3 bladen terug.
  De functie vergelijkt eerst met de huidige wortel en loopt dan de 64 wortels
  van de changelog af, oudste vooraan; deze wortel is de 61e.
- **afwijzing buiten het venster**: alleen `accepts_root` voor een digest die
  nooit een wortel was: vergelijking met de huidige wortel en alle 64
  plaatsen, dan weigering.

**De vorige versie van de kolom "verifiëren met oudere wortel in het venster"
was ongeldig.** De oude tabel staat in de versie van dit bestand in commit
`54ed7b8`; de getallen van die kolom zijn hier bewust niet overgenomen. Die meting
maakte de bewijzen tegen de huidige wortel en controleerde ze met `verify_at`
tegen een wortel van 3 bladen terug. Dan vouwt geen enkel bewijs naar die
wortel: 0 van de 256 controles slaagden, bij alle drie de boomgroottes
(nagemeten op 2026-11-16 met de code van toen), en `accepts_root` werd door
de kortsluiting van `&&` nooit aangeroepen. De meting mat dus hoe lang een
mislukte vouwing duurt.

Waardebehoud (`Shape::check` over 33 niveaus): 20 ns in alle drie de runs.

### Besteding (`meet_besteding`)

| bladen | tijd per besteding |
|---|---|
| 1.000 | 44,4–80,1 µs |
| 10.000 | 44,1–45,3 µs |
| 100.000 | 46,5–47,3 µs |

### Post-kwantumhandtekeningen (`meet_pq_vloer`, `meet_falcon_vloer`)

| schema | tekenen | verifiëren |
|---|---|---|
| FN-DSA-512 | 268–491 µs | 27–52 µs |
| FN-DSA-1024 | 538–557 µs | 50–53 µs |
| ML-DSA-44 | 146–281 µs | ≈ 30–80 µs * |
| ML-DSA-65 | ≈ 200–410 µs * | ≈ 50–120 µs * |
| ML-DSA-87 | 209–290 µs | 82–84 µs |

\* Hoogste waarde meer dan twee keer de laagste; daarom afgerond op
tientallen.

De tijden van ML-DSA wisselen sterk tussen de runs. ML-DSA-87 tekende in twee
van de drie runs sneller dan ML-DSA-65. De verificatietijden van ML-DSA-44 en
-65 lagen in twee van de drie runs ruim twee keer hoger dan in de derde. Ook
FN-DSA-512 wisselde: de hoogste tijd was bij tekenen 1,8 keer de laagste (491
tegen 268 µs) en bij verifiëren 1,9 keer (52 tegen 27 µs). De oorzaak van deze
spreiding is niet onderzocht. Mogelijk speelt mee op welke kern de meting
toevallig draaide (de machine heeft 10 snelle en 10 zuinige kernen, en de
threads zijn niet vastgezet; zie boven); mogelijk ook afwijzingsbemonstering bij
het tekenen, waardoor een handtekening meer pogingen kan vragen.

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
