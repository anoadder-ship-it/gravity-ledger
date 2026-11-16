# gravity-ledger — STATUS.md

**Doel:** eerste bestand bij hervatten. Legt vast waar we staan, wat bewezen is,
en welke keuzes waarom zijn gemaakt — zodat niets herhaald hoeft te worden.

Laatst bijgewerkt: 2026-11-16. **107 tests groen (57 in de bibliotheek, 29 in
`tests/besteding.rs`, 9 in `tests/veiligheid.rs`, 8 in `tests/handtekening.rs`,
4 doctests), plus 4 metingen onder `#[ignore]`; alle
prestatiegetallen gemeten, niet geschat.** Externe review van 2026-11-10
verwerkt in sectie 0; stand van de bevindingen in sectie 0a.

---

## 0. Besluit van de eigenaar (10 november 2026)

Naar aanleiding van de externe review van de stand vóór stap 1 (bevindingen G1–G6,
historisch verslag met de oorspronkelijke code in `docs/review-2026-11-10.md`):

1. **Veiligheid gaat altijd boven privacy.** Een eigenschap die geld kan laten
   ontstaan of stelen, wordt nooit ingeruild tegen anonimiteit.
2. **Eerst een zichtbare, controleerbare besteding.** De besteder toont
   commitment, positie, lidmaatschapsbewijs en opening. De keten controleert
   lidmaatschap (tegen een wortel in het venster), niveau en nullifier. Er is
   geen anonimiteit, maar alles is controleerbaar met alleen hashes.
3. **De eigenaar komt in de noot en de besteding wordt getekend.** De
   commitment bevat de hash van een publieke sleutel. De besteding wordt met
   die sleutel getekend over nullifier en uitgang.
   *Technische keuze (zelfde dag):* Ed25519 achter een trait, met een gepinde,
   actuele versie van een doorgelichte bibliotheek en `cargo audit` in de
   controle. Geen `falcon-rs` in productiecode. De noot bevat H(pk); de sleutel
   wordt pas bij besteding getoond. Falcon komt later achter dezelfde trait,
   zodra er een doorgelichte bron of een Solana-precompile is. Zichtbare
   besteding en handtekening gaan alleen samen live.
4. **Anoniem besteden blijft onderzoek** tot er een bruikbaar post-kwantum
   één-van-N-bewijs bestaat. Het model uit 3d veronderstelt een vertrouwde
   verifieerder (zie de aantekening daar).

Gevolg voor sectie 1: "dubbeluitgave wordt ontdekt" geldt pas als ook
lidmaatschap wordt gecontroleerd. Sinds stap 7 doet `Ledger::besteed` dat; de
oude `spend`, die een nooit opgenomen noot kon besteden (review G1, R1/R2), is
verwijderd.

## 0a. Stand van de reviewbevindingen (12 november 2026, met de rondes van 15 en 16 november)

| stap | wat | bevinding |
|---|---|---|
| 1 | `dsh` zet de lengte vóór de tag | R6/G5 gesloten |
| 2 | `live()` geeft een `Result`; `spend` weigert een lege emmer | R3/G4 gesloten |
| 3 | niveau publiek in het blad; `Ledger::is_lid` | — |
| 4 | hash van de publieke sleutel in de commitment | — |
| 4b | STATUS.md en SECURITY.md bij de stand | — |
| 5 | trait `Handtekening`, Ed25519 (`ed25519-dalek` =2.2.0), alleen `verify_strict` | — |
| 5b | tests kleine-ordesleutel en ongeldige sleutel | — |
| 6 | richting uit de index, geen vlag in het bewijs; velden `pub(crate)` | — |
| 7 | zichtbare, getekende besteding `Ledger::besteed`; oude `spend` weg | **R1 en R2 gesloten** |
| 7c | één weg om munten uit te geven: `Ledger::mint` telt in de emmer, `mint_to_ledger` roept het aan | review van stap 7 (verdringing in een emmer) gesloten |
| 7d | documentatie bij de stand na 7c, plus een `compile_error!` voor niet-64-bit doelen (in 7e teruggedraaid) | — |
| 7e | de index gaat als u64 (8 byte, little-endian) in het blad; `MAX_DIEPTE` vast op 64; wortels en bewijzen voor n = 1..70 byte-gelijk aan die van stap 7 (gemeten door de reviewer) | `compile_error!` weg; wasm32 compileert |
| 7f | test `alice_na_mallory_in_dezelfde_emmer`; dieptetest op `MAX_DIEPTE`; docstring `Ledger::mint`; open punt 13 | vier lage punten uit een onafhankelijke review van stap 7c tot en met 7e. De dieptetest (`stap6_te_lang_bewijs_wordt_geweigerd`) wordt zonder de dieptegrens alleen in een debug-build rood; in een release-build overleeft de mutant omdat de schuif daar omloopt, zonder veiligheidsgevolg. De test borgt de grens dus alleen in debug. |

De commit-voor-commit geschiedenis van deze stappen, met de rood-vóór-groen-uitkomsten, is niet gepubliceerd; de tests in tests/ en src/ bewijzen de eigenschappen.

**Gewijzigde verwachting in stap 7c.** De eerste versie van deze wijziging kwam
niet van de schrijver van de rest van de code en is pas overgenomen na
volledige nalezing, een rode run en een mutatiecontrole.

| | test | plek | verwachte uitkomst |
|---|---|---|---|
| oud (tot en met stap 7) | `lege_emmer` | `tests/besteding.rs` (stap 7) | een munt die alleen via `Ledger::mint` is opgenomen, geeft `Err(LegeEmmer)` |
| nieuw | `ledger_mint_telt_in_de_emmer` (omgekeerd, niet vervallen) | `tests/besteding.rs` | dezelfde munt telt in emmer (0,4) en de besteding slaagt; daarna `live` = 1 |
| neemt de taak van controle 8 over | `controle_8_lege_emmer_bij_inconsistente_tellers` | unit-test in `src/besteding.rs` | met een blad in boom en `minted` maar niet in `buckets`: `Err(LegeEmmer)`, toestand gelijk |

Reden: sinds 7c telt `Ledger::mint` de munt in zijn emmer. De oude verwachting
beschreef precies het gedrag waarmee een ongetelde munt de plaats van een
getelde munt van een ander kon innemen. Van buiten de crate kan de toestand
waarin controle 8 weigert niet meer ontstaan; de unit-test bouwt hem op via de
`pub(crate)`-velden, zonder nieuwe publieke functie.

**Mutaties van stap 7.** Eén wijziging tegelijk in `src/besteding.rs`, daarna
teruggezet. Gemeten in de review van 2026-11-12. Bij de eerste drie stopte
`cargo test` na het eerste rode testbestand; de testbestanden daarna zijn bij
die mutaties niet gedraaid.

| controle | mutatie | rode tests | gemeten op |
|---|---|---|---|
| 4 (venster) | de regel met `accepts_root` weg | `wortel_buiten_het_venster`, `record_van_keten_a_op_keten_b` | stap 7 |
| 6 (nullifier) | `index_in` uit N | `alleen_index_gewijzigd_naar_zelfde_c`, `bericht_volgens_specificatie`, `zelfde_c_op_twee_posities` | stap 7 |
| volgorde 6–9 | N al bij controle 6 verbranden, vóór controle 7 | `alleen_index_gewijzigd_naar_zelfde_c`, `chain_gewijzigd_na_tekenen`, `elke_bit_van_de_handtekening`, `c_uit_gewijzigd_na_tekenen`, `niveau_gewijzigd_na_tekenen`, `geen_wijziging_bij_weigering`, `lege_emmer` (sinds 7c omgekeerd), `record_van_keten_a_op_keten_b`, `robuustheid_tienduizend_records` | stap 7 |
| 7 (bericht) | `niveau` uit M | `bericht_volgens_specificatie`, `niveau_gewijzigd_na_tekenen` | stap 7 |
| 8 (emmer) | de regel met `live(…) == 0` weg | `controle_8_lege_emmer_bij_inconsistente_tellers`, op de eis `Err(LegeEmmer)` | stap 7c |

Controles 1, 2, 3 en 5 en de handtekeningverificatie zelf zijn in de review
van 2026-11-12 niet gemuteerd, en in de ronde van 2026-11-15 hieronder ook
niet (open punt 17).

- **R1 en R2 zijn gesloten in stap 7.** Stap 2 sloot alleen het geval van een
  lege emmer. Sinds stap 7 rekent de keten zelf C_in en het blad uit en
  controleert lidmaatschap; de tests `r1_nooit_uitgegeven_munt` en
  `r2_zelfde_geheim_ander_epoch` (`tests/besteding.rs`) eisen `GeenLid`. Beide
  waren eerst rood tegen de oude `spend` van stap 6.
- R4 en R5 waren geen fouten; ze staan nu als asserts (R4 in
  `tests/veiligheid.rs`, R5 sinds stap 6 in de unit-tests van `src/tree.rs`).
- De controle dat een blad gelijk is aan H(index, niveau, commitment) zit in
  controle 5 van `Ledger::besteed`: de keten rekent `blad(niveau, C_in)` zelf
  uit; een kale commitment in de boom is geen lid.

**Ronde van 15 november 2026.** Geen wijziging aan productiecode of aan de
openbare API.

- Commentaar en rustdoc nagelopen op beweringen die de code niet waarmaakt,
  en hersteld (`bench.rs`, `conservation.rs`, `lib.rs`, `note.rs`,
  `spend.rs`, `tree.rs`).
- `g4_besteden_uit_lege_emmer_faalt` eist nu precies `GeenLid` en een
  ongewijzigde ledger, in plaats van alleen `is_err()`. Proef met controle 4
  die altijd weigert: de oude g4 bleef groen, de nieuwe wordt rood.
- Mutatie van controle 8 (de regel met `live(…) == 0` weg), vóór en na de
  aanscherping van g4, met `--no-fail-fast`: alleen
  `controle_8_lege_emmer_bij_inconsistente_tellers` wordt rood; g4 blijft
  groen. g4 bewaakt R3 dus niet (open punt 16).
- Besluit over `Shape::check`: hij blijft openbaar en ongewijzigd;
  `Ledger::besteed` roept hem niet aan, omdat hij voor de vorm van een
  besteding (één invoer en één uitgang op hetzelfde niveau) niets kan
  weigeren. Dat staat nu in de rustdoc van `src/conservation.rs`.
- Nieuwe test `een_invoer_een_uitgang` (`src/conservation.rs`): de vorm met
  één invoer en één uitgang wordt op alle 33 niveaus aanvaard en voor alle
  1.056 paren verschillende niveaus geweigerd, met de onbalans op het
  laagste niveau. Mutatie (`check` laat elke onbalans door, `--no-fail-fast`):
  rood worden `een_invoer_een_uitgang`,
  `extra_noot_op_elk_niveau_wordt_ontdekt` en `r4_behoud_omgekeerd`.

**Ronde van 16 november 2026.** Geen wijziging aan productiecode of aan de
openbare API.

- De venstermeting in `meet_ladder_en_bewijzen` mat een verificatie die
  altijd faalde: de bewijzen waren tegen de huidige wortel gemaakt en werden
  tegen een oudere gecontroleerd (0 van de 256 slaagden). Nu worden bewijs en
  wortel op hetzelfde moment vastgelegd, en er is een gemeten geval bij
  gekomen van een waar bewijs tegen een wortel net buiten het venster, dat
  wordt geweigerd. Elke getimede controle in `src/bench.rs` eist haar
  uitkomst; met de oude fout teruggezet wordt de meting rood. Opnieuw gemeten
  in drie runs; zie sectie 3 en `docs/metingen.md`.
- Commentaar: de rustdoc van `DEFAULT_CHANGELOG` zegt precies wat het
  venster bevat (de huidige wortel en de 63 daarvoor); in `src/note.rs` zijn
  "QROM-binding", "PQ voor gratis" en "botsingsvast en verborgen" vervangen
  door de gebruikte hash en de aannames waarop binding en verborgenheid
  steunen; punt 13b hieronder spreekt niet meer van een audit.

---

## 1. Huidige staat

**Bewezen werkend (getest, gemeten):**
- Hashlaag op BLAKE3-XOF, 64 byte, met domeinscheiding. Known-answer-test tegen
  de officiële BLAKE3-vector; de XOF-prefix wordt expliciet vergeleken met de
  256-bit output, zodat de XOF-aanroep aantoonbaar BLAKE3 doet.
- Incrementele Merkle-accumulator: `push` O(log n), `root` O(1), `proof`
  O(log n); tijden in `docs/metingen.md`.
- Equivalentietest: cached wortel == trage volledige herbouw voor n = 0..40.
- Aanvallen uitgesloten: bitverdraaiing in elke sibling op elk niveau
  (`elke_siblingbitverdraaiing_faalt`: 12 posities in een boom van 12 bladen,
  elk van de 4 niveaus van het bewijs, de 8 bits van de eerste byte van de
  sibling; 384 gevallen), verkeerde index, verkeerde bladeren, verkeerde
  wortel, index buiten bereik.
- Waardebehoud als telling over 33 denominatieniveaus (`Shape::check`; tijd
  in `docs/metingen.md`). `Ledger::besteed` roept `Shape::check` niet aan.
- Dubbeluitgave wordt ontdekt via nullifier N = H("GL/nullify/v1", chain,
  index_in, C_in), door de keten uitgerekend (stap 7): `besteding::nullifier`.
  `Note::nullifier` (v0) in `src/note.rs` is onderzoek (het anonieme model uit
  3d) en speelt bij de besteding geen rol.
- **Zichtbare, getekende besteding** (stap 7, `src/besteding.rs`): record v1,
  negen controles in vaste volgorde, de toestand verandert pas na de laatste.
  Ed25519 met `verify_strict` over een digest van 64 byte. Voor de controles
  in de mutatietabel van sectie 0a is aangetoond dat ze nodig zijn: haal je er
  één weg, dan wordt een test rood. 10.000+ willekeurige en verminkte records
  geven nooit een paniek en wijzigen de toestand nooit.
- Bewijsformaat (stap 6): alleen siblings, 64 byte per niveau; de richting volgt
  uit de index, dus geen enkele bitverdraaiing van een echt bewijs wordt nog
  aanvaard. Een pad langer dan `MAX_DIEPTE` (64) wordt geweigerd. De velden van
  `Ledger` en `Accumulator::push` zijn `pub(crate)`; een blad komt alleen via
  `blad()` in de boom. Van buiten de crate verandert de toestand van een
  ledger alleen via `Ledger::mint` (ook via `mint_to_ledger`, dat het
  aanroept) en `Ledger::besteed`.

**Niet gebouwd, niet beweren:** anonieme bestedingslaag, Solana-programma,
netwerk-integratie, post-kwantum handtekening (Falcon) in productiecode.

**Recordformaat v1** (zichtbare besteding; getallen u64 little-endian):

| veld | byte |
|---|---|
| versie (`0x01`) | 1 |
| schema (`0x01` = Ed25519) | 1 |
| index_in | 8 |
| niveau (< 33) | 1 |
| epoch_in | 8 |
| geheim_in | 32 |
| pk | 32 |
| wortel (in het venster) | 64 |
| diepte (≤ 64) | 1 |
| bewijs | 64 × diepte |
| C_uit | 64 |
| handtekening | 64 |
| **totaal** | **276 + 64 × diepte** |

Getekend: `M = dsh("GL/besteding/v1", [chain, versie, schema, N, niveau,
C_uit])`; de index via N; wortel en bewijs niet. Gemeten: **916 / 1.172 /
1.364 byte** bij 1.000 / 10.000 / 100.000 bladeren; de tijd per besteding staat
in `docs/metingen.md`.

---

## 2. Ontwerpkeuzes en hun onderbouwing

**2.1 Eén primitief, 64 byte digest.**
Eerste versie: handgeschreven SHA-256 (verwijderd — geen eigen cryptografie).
Tweede versie: `BLAKE3-256 ‖ SHA3-512`, 96 byte, "moet beide breken". Dat was
onnodig. De bekende kwantum-botsingsaanval van Brassard, Høyer en Tapp (BHT)
vindt bij een hash met n bit uitvoer een botsing met ongeveer 2^(n/3)
bevragingen: voor n = 256 ongeveer 2^85 (256/3 ≈ 85,3), voor n = 512 ongeveer
2^171 (512/3 ≈ 170,7). Volgens die rekensom vraagt een grotere marge om een
*bredere* output, niet om twee functies; voor BLAKE3-XOF op 64 byte geeft de
som ongeveer 2^171, met één afhankelijkheid en 33% kleinere bewijzen. Dit is een rekensom op
een algemene grens van die ene aanval, geen bewijs en geen meting; deze repo
bevat geen veiligheidsbewijs. Echte scheiding zit in domeinscheiding, niet in
het stapelen van primitieven.

**2.2 Index-gebonden bladeren.**
Zonder index-binding had mijn eigen boom twee echte gaten (beide aangetoond met
een falende test vóór de fix): `[a,b,c]` en `[a,b,c,c]` kregen dezelfde wortel,
en een bewijs bond zijn positie niet. Met `leaf = H(tag, index, commitment)`
zijn beide dicht.

**2.3 Incrementele boom met behoud van pairing.**
De cache mag de boom niet veranderen: pairing (0,1),(2,3),… en dupliceren van
een los kind zijn identiek aan de referentie-implementatie, en de
equivalentietest afgedwongen. Twee bugs in de eerste incrementele versie
(extra knoop bij n=1; vervangen in een leeg niveau) waren zonder die test stille
wortel-fouten geworden.

**2.4 Eén handtekening per transactie, niet per noot.**
Gemeten: de kleinste PQ-handtekening (FN-DSA-512) is 809 byte in het
constant-time-formaat van `falcon-rs` (666 byte in het padded-formaat), ML-DSA-44
is 2.420 byte, terwijl het hele lidmaatschapsbewijs 1.088 byte is (100.000
bladeren, gemeten na stap 6). De handtekening
bepaalt de ondergrens; per noot signeren vermenigvuldigt die.

**2.5 Hypothese (niet gebouwd, niet gemeten): anonimiteit zou relatief
goedkoop kunnen zijn.** De PQ-handtekening betaal je hoe dan ook. De
verwachting is dat een bewijs dat verbergt welk blad je besteedt, daar
bescheiden bovenop komt en niet een veelvoud kost. Of dat klopt, hangt af van
een één-van-N-bewijs dat er nog niet is (sectie 6, punt 1).

---

## 3. Gemeten getallen

Alle tijden (mint, wortel, bewijzen, verifiëren, venster, waardebehoud,
besteding, post-kwantumhandtekeningen) staan in `docs/metingen.md`: drie runs
op één machine, met de laagste en hoogste waarde. Hieronder alleen maten; die
zijn vast.

| bladen | diepte | bewijs |
|---|---|---|
| 1.000 | 10 | 640 byte |
| 10.000 | 14 | 896 byte |
| 100.000 | 17 | 1.088 byte |

Bewijs: 64 byte per niveau (sinds stap 6; daarvoor 65).

**Post-kwantum handtekeningen, gemeten (zelfde harness):**

| | publieke sleutel | signatuur |
|---|---|---|
| FN-DSA-512 (Falcon) | 897 byte | **809 byte** |
| FN-DSA-1024 (Falcon) | 1.793 byte | 1.577 byte |
| ML-DSA-44 | 1.312 byte | 2.420 byte |
| ML-DSA-65 | 1.952 byte | 3.309 byte |
| ML-DSA-87 | 2.592 byte | 4.627 byte |

De Falcon-handtekeningen zijn in het constant-time-formaat dat `falcon-rs`
0.3.1 teruggeeft (`FALCON_SIG_CT`, kopbyte `0x50 + logn`; zie
`docs/metingen.md`). In het padded-formaat zijn ze 666 en 1.280 byte; dat zijn
ook de groottes in draft-ietf-cose-falcon-04, tabel 1.

SIMD-0461 was een voorstel om Falcon-verificatie als syscall aan Solana toe te
voegen; het is gesloten en niet doorgevoerd. Gemeten is FN-DSA-512 wel de
kleinste van de vijf: **drie keer kleiner dan ML-DSA-44**. Budget van de zichtbare
besteding (stap 7) bij 100.000 bladeren: met Ed25519 **1.364 byte** (gemeten);
met FN-DSA-512 in plaats van Ed25519 (pk 897 + handtekening in plaats van
32 + 64) **2.974 byte** met 809 byte (CT) of **2.831 byte** met 666 byte
(padded), berekend, tegen een limiet van 4.096 (SIMD-0296, live sinds
15 september 2026 (epoch 1035)). Bij 2^32 bladeren is dat 3.934 of 3.791 byte,
plus een overhead die niet gemeten is (open punt 12).

**Acceptatievenster (concurrency):** gemeten op 2026-11-16, drie runs (de
tweede meetronde van die dag, `docs/metingen.md`). Een waar bewijs tegen de
wortel van 3 bladen terug werd in alle drie de runs aanvaard (10.240 van de
10.240 controles per boomgrootte); een waar bewijs tegen de wortel van 64
bladen terug werd steeds geweigerd. Verificatie in het venster kostte per run
0,04 tot 0,15 µs meer dan verificatie tegen de huidige wortel (1.000 bladen:
2,09–2,12 tegen 1,99–2,01 µs; 10.000: 2,81–2,84 tegen 2,74–2,75 µs; 100.000:
3,36–3,45 tegen 3,30–3,32 µs); `accepts_root` alleen kostte 80–91 ns. Gemeten is alleen een wortel op de 61e van
de 64 plaatsen in de scan; andere posities niet. De vorige uitspraak hier
("kost weinig meer") steunde op een ongeldige meting die een altijd falende
verificatie mat; zie `docs/metingen.md`.

Caveats: de tijden zijn release-builds op één machine, 30 herhalingen voor PQ.
De tijden van ML-DSA wisselden sterk tussen de runs. De oorzaak is niet
onderzocht: mogelijk de kern waarop de meting toevallig draaide (snelle en
zuinige kernen, threads niet vastgezet), mogelijk afwijzingsbemonstering bij
het tekenen. Komt de spreiding uit het algoritme, dan telt ze mee bij harde
bloktijd-deadlines.

---

## 3b. Waarom we níet op Light Protocol liften

Light Protocol (ZK Compression) is de volwassen buur: drie programma-audits
(OtterSec, Neodyme, Zellic), circuits die door Reilabs formeel zijn
geverifieerd, en productiegebruik voor cNFTs.
Toch liften we er niet op, en de reden staat in hun eigen afhankelijkheden:

- **`light-poseidon` hangt aan `ark-bn254`.** Hun boom hashet Poseidon over het
  BN254-veld, speciaal gebouwd om in een SNARK geverifieerd te worden. Knopen
  zijn veld-elementen van ~32 byte.
- **Het protocol wordt geleverd met gnark-toetsen per boomhoogte**, dus
  SNARK-gebaseerde geldigheidsbewijzen en bijbehorend setup-apparaat.

Onze bindende commitment is precies dát wat niet verwaterd mag worden: een
botsing in de boom is nieuw geld uit het niets. Bij 32-byte knooppunten is dat
klassiek netjes, maar volgens de BHT-grens uit 2.1 (ongeveer 2^(n/3)
bevragingen) is een botsing bij 256 bit te vinden met ongeveer 2^85 — de reden
om naar 64-byte BLAKE3-XOF-knooppunten te gaan (ongeveer 2^171). Ook dit is een
rekensom op een algemene grens, geen bewijs en geen meting. En een SNARK met setup importeren
in een project waarvan het bestaansrecht is dat het géén SNARK nodig heeft, is
het doel opofferen aan het middel.

Hun stack is geoptimaliseerd voor een eigenschap die wij expres niet willen.
Dat is geen trots, dat is een rekensom. **Wat we wél overnemen: hun audit-lat
(drie audits, niet nul) en hun inzicht dat één boom veel gelijktijdige schrijvers
moet kunnen hebben.**

## 3c. Concurrentie: acceptatievenster in plaats van bewijstransformatie

Het probleem: twintig transacties in één slot willen allemaal noten minten of
besteden. Een bewijzer die wortel R zag, heeft bij binnenkomst een boom met
wortel R' — en mag alsnog niet botsen met negen andere schrijvers.

**Eerste poging (verworpen):** de change-log gebruiken om een bewijs van R naar
R' te *transformeren*, zoals Light doet. Twee defecten, beide gevonden door de
grondbewijstest over alle combinaties n×m vóórdat iets groen was:
1. het opgeslagen pad bevatte de wortel zelf, terwijl een wortel nooit een
   sibling is — élke update liep tegen de "geef niets terug"-tak;
2. een duplo-sibling (knoop met zichzelf aan de rechterrand van een oneven
   niveau) stond als "links" genoteerd; werd die sibling later een écht buur-
   knoopje, dan verving ik de waarde maar niet de richting, en vouwde het bewijs
   verkeerd.

**Waarom die complexiteit er niet hoefde:** de accumulator is **append-only**.
Een blad dat in wortel R zat, zit over tien updates nog steeds in de boom —
bladeren worden hier nooit verwijderd of herschreven. Een bewijs tegen een
iets oudere wortel bewijst dus nog steeds iets waars. De enige vraag is of we
die wortel nog kunnen verantwoorden.

**De oplossing daarom (28 regels):** `Change { root, index }` in een ring van
laatst-geziene wortels (`DEFAULT_CHANGELOG = 64`), plus
- `accepts_root(root)` — huidige wortel, of een uit het venster;
- `verify_at(proof, index, leaf, root)` — vouw naar het aangereikte wortel én
  dat wortel valt binnen het venster.

Buiten het venster wordt geweigerd: niet omdat het bewijs onwaar zou zijn, maar
omdat we dan geen begrensd verhaal meer hebben. Falen boven gokken. Kosten:
zie `docs/metingen.md`.

**Les voor de methode:** de test die ik vóór de implementatie schreef (n×m
combinaties, inclusief de groei-overgangen 1→2, 3→4, 7→8) vond beide defecten
direct. Een tweede les: mijn eerste meting van dit venster mat de huidige
wortel, die via de eerste toets meteen terugkeert. De echte scan kost een
veelvoud daarvan. Meet het pad dat je bedoelt, niet het dichtstbijzijnde.
Een derde les: de meting die daarna kwam, maakte bewijzen tegen de huidige
wortel en controleerde ze tegen een oudere; geen enkele controle slaagde, en
toch leverde de meting tot 2026-11-16 een getal op. Sindsdien eist elke
getimede controle in `src/bench.rs` haar uitkomst.

## 3d. Anonimiteitsmodel: menigte in de emmer (onderzoek, historisch)

> **Aantekening 2026-11-10 (besluit sectie 0): dit model veronderstelt een
> vertrouwde verifieerder.** Uit een record van alleen nullifier en commitment
> kan een keten niet vaststellen dat de nullifier bij een opgenomen noot hoort,
> en ook niet dat in- en uitgang hetzelfde niveau hebben. In de code werkt het
> alleen omdat `spend` de volledige noten met hun geheim krijgt. "Nul extra
> bytes" geldt dus alleen met zo'n verifieerder. Zonder vertrouwde verifieerder
> is een post-kwantum één-van-N-bewijs nodig, en dat bestaat niet. Dit model is
> daarom onderzoek, geen protocol. Het protocol is de zichtbare besteding uit
> sectie 0.
>
> **Aantekening 2026-11-12 (stap 7):** de code van dit model (`spend`, het
> `Spend`-record van 128 byte en `DOELMENIGTE`) is uit de bibliotheek
> verwijderd; ze bestond tot en met stap 6 (die geschiedenis is niet
> gepubliceerd). De getallen
> hieronder zijn met die code gemeten.

**Wat ik eerst dacht en waarom het niet deugt.** Als optie voor de bestedings-
laag stond erin: "begrensde sets, de hele set meesturen — eenvoudig, PQ-veilig".
Dat is twee keer onjuist. Grootte: 1.000 commitments à 64 byte is 64 KB tegen
een limiet van 4.096 — vijftien keer te groot. Erger: het model anonimiseert
niets. Wie zijn eigen opening meestuurt, geeft de verifieerder de mogelijkheid
te hashen en te kijken welke notitie het was. De anonimiteit verdwijnt op het
moment dat je hem bewijst.

**Wat wél kan met alleen hashes en PQ-handtekeningen.** Een besteding maakt
publiek: één nullifier (`Note::nullifier`, v0) en één nieuwe commitment. Dat
die nullifier zonder het geheim niet aan een commitment te koppelen is, is
geen vastgestelde eigenschap maar een aanname: een onvoorspelbaar geheim en
een hash die niets over zijn invoer prijsgeeft (rustdoc in `src/note.rs`);
deze repo bevat er geen bewijs voor. Een waarnemer ziet dus alleen: in emmer (epoch, niveau)
is er één verbrand en één bijgekomen. Het aantal noten van dát niveau in dát
epoch bepaalt hoeveel kandidaten hij heeft om input aan output te koppelen.

**De eigenschap die dit bruikbaar maakt: de anonimiteitset kost nul extra
bytes.** Hij groeit met gebruik, niet met transactiegrootte. Vergelijk dat met
het set-model, waar een tienmaal zo grote set tienmaal zoveel bytes kostte.

*Historisch, de code is verwijderd:* de tabel hieronder is gemeten met de code
van dit model tot en met stap 6; die code bestaat niet meer.

| gemeten | waarde |
|---|---|
| bestedingsrecord (nullifier + commitment) | **128 byte** |
| + FN-DSA-512 handtekening | 937 byte |
| + ~200 byte instructie-/accountoverhead | **1.137 byte van 4.096** |
| kandidaten bij emmer van doel 100 | 101 (de +1 is de zojuist geminte output) |

**De knop is dus een gebruiksknop, en die is een productkeuze:** in dit model
was dat `DOELMENIGTE = 100`; die constante stond tot en met stap 6 in
`src/spend.rs` en bestaat niet meer. Daaronder is een besteding bij benadering koppelbaar;
de code dwingt niets af, want de keten kan niet weten hoeveel opnames "genoeg"
zijn. Wat wél afgedwongen wordt: niveauvastheid per besteding, en dubbeluitgave.

**Twee grootheden, niet één.** Mijn eerste testbewering ("besteden houdt de
emmer gelijk groot") faalde, en kwam door mijn eigen redenering: een besteding
voegt wél een commitment toe (opnames +1) en haalt één besteedbare notitie weg
(live −1). Opnames zijn de kandidatenlijst voor "welke output is van mij", live
die voor "welke input verbrandde dit". Beide staan nu expliciet in de code
(`menigte` en `live`).

**Wat het model niet is:** geen sterke unlinkability. Het is menigte-anonimiteit
binnen (epoch, niveau) — sterker naarmate de emmer voller is, onder de
aannames hierboven. Er is geen audit-laag en geen auditsleutel:
`Note::audit_tag` bestaat, wordt alleen in de tests van `src/note.rs`
aangeroepen, en hoe een auditor een tag herkent staat niet in de code. Voor
onkoppelbaarheid zonder aannames is nog steeds een één-van-N-bewijs nodig, en dat
bestaat post-kwantum niet in geauditeerde vorm.

## 4. Afhankelijkheden en veiligheid

| crate | gebruik | status |
|---|---|---|
| `blake3` 1.8.6 | productie, enige primitief | officiële implementatie door de BLAKE3-auteurs; een onafhankelijke audit is niet vastgesteld |
| `ed25519-dalek` =2.2.0 | productie, handtekening (stap 5), zonder standaardfeatures, alleen `verify_strict` | exact gepind; cargo audit zonder meldingen (lokale database van 2026-09-24); RUSTSEC-2024-0344 opgelost vanaf 4.1.3 en `Cargo.lock` bevat 4.1.3; RUSTSEC-2022-0093 opgelost vanaf 2.0 |
| `sha2` 0.10.9 | dev-only (KAT-vectoren); sinds stap 5 ook in productie via `ed25519-dalek` | RustCrypto |
| `falcon-rs` 0.3.1 | dev-only (PQ-meting, FN-DSA/FIPS 206) | niet geauditeerd — alleen meten, niet tekenen |
| `dilithium-rs` 0.4.1 | alleen dev-afhankelijkheid (PQ-meting) | volgens het eigen `SECURITY_AUDIT.md` van het crate: ronde 1 (v0.2.0, 2026-07-20) 11 bevindingen, ronde 2 (v0.4.0, 2026-08-31) 12 bevindingen, alle gemarkeerd als opgelost (één in ronde 1 als "Documented"); High waren F1 in ronde 1 (de `simd`-feature compileerde niet) en R2-1 in ronde 2 (verkeerde HashML-DSA-OID); `chknorm` en `make_hint` (R2-5, R2-6) zijn Low. Of de audits onafhankelijk waren, is niet vastgesteld. |

**`ml-dsa` (RustCrypto) wordt niet gebruikt.** Die crate meldt zelf nooit
onafhankelijk geauditeerd te zijn en had CVE-2026-24850 / GHSA-5x2r-hc65-25f9
(verificatie aanvaardde handtekeningen met herhaalde hint-indices; getroffen
≥0.0.4, opgelost vanaf 0.1.0-rc.4). Deze advisory gaat over `ml-dsa`, niet over
`dilithium-rs`. Er was dus niets te upgraden — er stond geen `ml-dsa` in
`Cargo.toml` noch in de lockfile. Bewust niet toegevoegd.

**Aanbevolen voor de signatuur-laag: hybride FN-DSA (Falcon) + Ed25519.** Een
fout in één van beide laat de transactie dan nog klassiek of post-kwantum
beveiligd.

---

## 5. Onderzoek, 2026-09-23 (invloed op het ontwerp)

- **PQ-ringhandtekeningen zijn actief onderzoek, geen productie.** Voorbeelden:
  "DualRing-PRF: Post-Quantum (Linkable) Ring Signatures from Legendre and
  Power Residue PRFs" (IACR ePrint 2024/985) en "Post-Quantum Linkable
  Hash-Based Ring Signature Scheme for Off-Chain Payments in IoT" (MDPI
  Sensors, 2025). Er is wél een "één van de N"-primitief in onderzoek; er is
  géén geïmplementeerde, geauditeerde.
- **Light Protocol** bouwt en gebruikt concurrent Merkle trees en ZK Compression
  op Solana (productie voor cNFTs). Voordat hier nog een boom geschreven wordt:
  vergelijken.
- **Token-2022 Confidential Transfers / Confidential Balances** lossen "bedrag
  verbergen" al op met Twisted ElGamal en on-chain geverifieerde ZK-bewijzen —
  klassiek, niet post-kwantum, maar bestaand en getest.

---

## 6. Openstaand

0. **Zichtbare, getekende besteding** — **gebouwd in stap 7**
   (`Ledger::besteed`, record v1, Ed25519).
1. **Anonieme bestedingslaag**: **onderzoek** (besluit sectie 0). Het model in
   3d gaat uit van een vertrouwde verifieerder. Optie "hele set meesturen" is
   vervallen: te groot én niet anoniem. Een anonieme besteding wacht op een
   bruikbaar PQ-één-van-N-bewijs.
2. **Solana-integratie.** Nog geen code. Eerst de vergelijking met Light
   Protocol; daarna pas beslissen of eigen code nodig is.
3. **Falcon achter de trait `Handtekening`.** Ed25519 is gebouwd (stap 5).
   FN-DSA (Falcon) volgt zodra er een doorgelichte bron of een
   Solana-precompile is; `falcon-rs` is niet geauditeerd. Bij 100.000
   bladeren is het budget 2.974 byte (CT, 809 byte) of 2.831 byte (padded,
   666 byte), berekend, tegen 4.096; bij 2^32 zie punt 12 en sectie 3.
   `cargo audit` draait lokaal (database van 2026-09-24), nog niet in een CI.
4. **Anonimiteitssetgrootte** — onderzoek (3d); `DOELMENIGTE` is in stap 7
   uit de code verwijderd.
5. **Concurrentie** — **opgelost**: acceptatievenster (3c); tijden in
   `docs/metingen.md`.
6. **Wie mag munten.** `Ledger::mint` en `mint_to_ledger` controleren niet wie
   een noot opneemt (en ook niet het niveau; een munt van niveau 33 is op te
   nemen maar niet te besteden). In de bibliotheek is dat de rol van de
   emitter. Op de keten is het een manier om waarde uit het niets te maken. Dit
   hoort bij het Solana-programma en wordt daar beslist.
7. **`Ledger::mint` telde niet in een emmer** — **opgelost in stap 7c**.
   Tot en met stap 7 telde alleen `mint_to_ledger` de munt in
   haar emmer. De review van stap 7 bewees met een test: Alice krijgt via
   `mint_to_ledger` een munt in emmer (5,4), Mallory via `Ledger::mint` een munt
   in dezelfde emmer; Mallory besteedt zijn munt, en daarna wordt Alices
   geldige besteding van haar eigen munt geweigerd met `LegeEmmer`. Er ontstond
   geen waarde, maar een ongetelde munt kon de plaats van een getelde munt van
   een ander innemen. Nu telt `Ledger::mint` zelf en roept `mint_to_ledger`
   alleen `Ledger::mint` aan; `twee_mintwegen_blokkeren_elkaar_niet` en
   `mint_en_mint_to_ledger_geven_dezelfde_ledger` (`tests/besteding.rs`)
   bewaken dat.
8. **Eén sleutel per munt** (privacy van de zichtbare besteding: pk en de
   koppeling invoer–uitgang zijn openbaar). Productkeuze, niet in de code.
9. **Epoch van de ledger** heeft nog geen setter; uitgangen komen nu in emmer
   (0, niveau).
10. **Wortel en bewijs zijn niet ondertekend.** Elke wortel in het venster
    (de laatste 64) geeft met zijn bewijs een geldig record voor dezelfde
    besteding, dus er bestaan tot 64 varianten van hetzelfde record met
    hetzelfde effect en dezelfde N. `hash(record)` mag daarom nooit als
    transactienummer dienen; de nullifier N wel.
11. **Platform en wortel.** Sinds stap 7e hasht het blad de index altijd als
    u64 little-endian (8 byte), en rekent `Proof::verify` de richting met u64
    en een vaste grens `MAX_DIEPTE` = 64 in plaats van `usize::BITS`. De wortel
    hangt dus niet meer af van de breedte van `usize`; op 64-bit zijn de bytes
    gelijk aan die van daarvoor (alle tests en vaste waarden ongewijzigd).
    `cargo check --target wasm32-unknown-unknown` slaagt. Er is niets op een
    32-bit doel gedraaid: dat de wortel daar gelijk is, volgt uit de code en
    uit de test `blad_hasht_index_als_8_byte_little_endian`, maar is daar niet
    gemeten. Op 32-bit kan een ledger bovendien hooguit `usize::MAX` bladen
    hebben; een record met een grotere `index_in` geeft `GeenLid`.
12. **Falcon-budget bij 2^32 bladen is krap**: 3.934 byte met de handtekening
    van 809 byte die `falcon-rs` geeft (constant-time-formaat), of 3.791 byte
    met 666 byte (padded-formaat), plus overhead, tegen een limiet van 4.096.
    Met 666 byte blijft er meer ruimte over (305 tegen 162 byte), maar de
    overhead is niet gemeten; of het past, is dus niet vastgesteld (zie punt 3
    en sectie 3).
13. **Drie oude zaken uit een onafhankelijke review** (stap 7f), niet
    gerepareerd. a en b zijn losse zaken zonder directe relatie met punt 6;
    c hoort bij punt 6.
    a. `note::denom_value` is publiek en geeft een paniek (`assert!`) bij
       niveau 33 of hoger.
    b. `Shape::total_value` kan in een debug-build overlopen (paniek); de
       functie wordt alleen in tests aangeroepen, en er is geen audit-laag.
    c. `Ledger::mint` controleert niveau en epoch niet: een munt met niveau 33
       of hoger telt wel in een emmer, maar is niet te besteden.
14. **`Violation::Overflow` is onbereikbaar.** Elke telling in `Shape` is een
    u32; het aanbod is hooguit 4 · (2^32 − 1) = 17.179.869.180 en de vraag
    hooguit 5 · (2^32 − 1) = 21.474.836.475, ver onder 2^64 − 1. De
    controles blijven als vangnet staan.
15. **Ongebruikte openbare functies in `conservation`.** `Shape::aggregate`
    wordt nergens aangeroepen, ook niet in tests. `conservation::zero` wordt
    alleen binnen `src/conservation.rs` aangeroepen (`Shape::default` en de
    tests daar), daarbuiten nergens. Beide blijven staan.
16. **`LegeEmmer` is van buiten de crate niet bereikbaar.** Redenering: een
    munt die controles 1 tot en met 7 haalt, is via `mint` of `besteed`
    opgenomen en in haar emmer geteld, en per positie slaagt hooguit één
    besteding; zolang ze onbesteed is, is `live` dus minstens 1. Dit steunt
    op botsingsvrijheid van de hash en is niet uitputtend getest. Controle 8
    wordt alleen bewaakt door de unit-test
    `controle_8_lege_emmer_bij_inconsistente_tellers`.
17. **Controles 1, 2, 3 en 5 zijn niet gemuteerd**, net als de
    handtekeningverificatie zelf (controle 7; alleen het bericht M is
    gemuteerd, zie sectie 0a).

Punt 6 (wie mag munten) blijft open.

---

## 7. Werken met deze repo

```bash
cargo test                                 # 107 tests, inclusief aanvallen
cargo test --release -- --ignored --nocapture  # metingen, zie docs/metingen.md
cargo test --offline                       # zonder netwerk
```

Werkregels die bij dit werk zijn opgedaan en helpen:
- **Geen handgeschreven cryptografie.** Bekende, gepinde crates; geen eigen cryptografie.
- **Aanvallen eerst laten falen, dan pas repareren.** Beide gaten in de boom
  waren onzichtbaar gebleven zonder die volgorde.
- **Metingen leven in de repo** (`src/bench.rs`, `#[ignore]`), niet in /tmp.
- **Een groene test is geen bewijs** — kijk waar het groen vandaan komt.
