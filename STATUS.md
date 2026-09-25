# gravity-ledger — STATUS.md

**Doel:** eerste bestand bij hervatten. Legt vast waar we staan, wat bewezen is,
en welke keuzes waarom zijn gemaakt — zodat niets herhaald hoeft te worden.

Laatst bijgewerkt: 2026-09-25. **57/57 tests groen; alle prestatiegetallen
gemeten, niet geschat.**

> **Over deze versie:** de vorige STATUS.md (28 KB) bevatte voor het grootste
> deel de geschiedenis van een ander project (SpankWallet: WebAuthn-passkeys,
> secp256r1-precompile, Phantom-routing, `spankwallet.so`-deploys). Die tekst is
> vervangen; de oude inhoud staat in de git-geschiedenis van dit bestand en is
> daar terug te halen.

---

## 1. Huidige staat

**Bewezen werkend (getest, gemeten):**
- Hashlaag op BLAKE3-XOF, 64 byte, met domeinscheiding. Known-answer-test tegen
  de officiële BLAKE3-vector; de XOF-prefix wordt expliciet vergeleken met de
  256-bit output, zodat de XOF-aanroep aantoonbaar BLAKE3 doet.
- Incrementele Merkle-accumulator: `push` O(log n), `root` O(1) (2 ns),
  `proof` O(log n) (0,17 µs), verificatie 4,1 µs bij 100.000 bladeren.
- Equivalentietest: cached wortel == trage volledige herbouw voor n = 0..40.
- Aanvallen uitgesloten: bitverdraaiing van elke sibling op elk niveau (12
  posities × 8 niveaus × 8 bits), verkeerde index, verkeerde bladeren, verkeerde
  wortel, index buiten bereik.
- Waardebehoud als telling: 19 ns per controle over 33 denominatieniveaus.
- Dubbeluitgave wordt ontdekt via nullifier (keten-gebonden).

**Niet gebouwd, niet beweren:** anonieme bestedingslaag, Solana-programma,
netwerk-integratie, signatuur-laag in productiecode.

---

## 2. Ontwerpkeuzes en hun onderbouwing

**2.1 Eén auditeerde primitief, 64 byte digest.**
Eerste versie: handgeschreven SHA-256 (verwijderd — geen eigen cryptografie).
Tweede versie: `BLAKE3-256 ‖ SHA3-512`, 96 byte, "moet beide breken". Dat was
onnodig: de kwantum-marge (BHT geeft ~n/3 bits) vraagt om een *bredere* output,
niet om twee functies. BLAKE3-XOF op 64 byte geeft dezelfde ~170-bit marge met
één afhankelijkheid en 33% kleinere bewijzen. Echte scheiding zit in
domeinscheiding, niet in het stapelen van primitieven.

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
Gemeten: de kleinste PQ-handtekening (FN-DSA-512) is 809 byte, ML-DSA-44 is dat
2.420 byte, terwijl het hele lidmaatschapsbewijs 1.105 byte is. De handtekening
bepaalt de ondergrens; per noot signeren vermenigvuldigt die.

**2.5 Anonimiteit is relatief goedkoop, en dat verandert de prioriteit.**
De PQ-handtekening betaal je hoe dan ook. Daar bovenop kost verbergen welk blad
je besteedt 1.105 byte — circa 31% extra, niet de factor 10 waar zulke ontwerpen
normalerwijs aan sneuvelen.

---

## 3. Gemeten getallen

| anonimiteitsset | mint | wortel | bewijs | verifiëren | bewijs maken |
|---|---|---|---|---|---|
| 1.000 | 2,6 µs | 2 ns | 650 byte | 2,6 µs | 0,04 µs |
| 10.000 | 3,3 µs | 2 ns | 910 byte | 3,5 µs | 0,06 µs |
| 100.000 | 4,1 µs | 2 ns | 1.105 byte | 4,1 µs | 0,17 µs |

Waardebehoud: 19 ns per controle. PQ-ondergrens: ML-DSA-44 2.420 byte / 69 µs
verifiëren; ML-DSA-65 3.309 byte; ML-DSA-87 4.627 byte.

**Post-kwantum handtekeningen, gemeten (zelfde harness):**

| | publieke sleutel | signatuur | signeren | verifiëren |
|---|---|---|---|---|
| FN-DSA-512 (Falcon) | 897 byte | **809 byte** | 480 µs | 51 µs |
| FN-DSA-1024 (Falcon) | 1.793 byte | 1.577 byte | 526 µs | 49 µs |
| ML-DSA-44 | 1.312 byte | 2.420 byte | 148 µs | 29 µs |
| ML-DSA-65 | 1.952 byte | 3.309 byte | 213 µs | 48 µs |
| ML-DSA-87 | 2.592 byte | 4.627 byte | 258 µs | 79 µs |

Solana noemt FALCON als richting (Anza, SIMD-0461 precompile) en dat is meetbaar
de juiste keuze: **drie keer kleiner dan ML-DSA-44**. Budget per besteding bij
FN-DSA-512: 809 + 1.105 = **1.914 byte** tegen een limiet van 4.096 (SIMD-0296,
live sinds 15 september 2026).

**Acceptatievenster (concurrency):** scan over de log kost **73–85 ns**,
afwijzing buiten het venster 79 ns; verificatie mét venster is gelijk aan
verificatie zonder (2,55 vs 2,66 µs — ruis). De laag is dus vrijwel gratis.

Caveats, expliciet: één run per meting, release-build, één kern, 30 iteraties
voor PQ. ML-DSA-tekentijden zijn variabel door afwijzingsbemonstering (264 µs
voor level 87, sneller dan 65) — dat is een eigenschap van het algoritme, geen
meetfout, en het telt mee bij harde bloktijd-deadlines.

---

## 3b. Waarom we níet op Light Protocol liften

Light Protocol (ZK Compression) is de volwassen buur: drie programmaraudits
(OtterSec, Neodyme, Zellic), formeel geverifieerde circuits, productiegebruik
voor cNFTs, `account-compression` 2.0, `light-poseidon` 12,4 miljoen downloads.
Toch liften we er niet op, en de reden staat in hun eigen afhankelijkheden:

- **`light-poseidon` hangt aan `ark-bn254`.** Hun boom hashet Poseidon over het
  BN254-veld, speciaal gebouwd om in een SNARK geverifieerd te worden. Knopen
  zijn veld-elementen van ~32 byte.
- **Het protocol wordt geleverd met gnark-toetsen per boomhoogte**, dus
  SNARK-gebaseerde geldigheidsbewijzen en bijbehorend setup-apparaat.

Onze bindende commitment is precies dát wat niet verwaterd mag worden: een
botsing in de boom is nieuw geld uit het niets. Bij 32-byte knooppunten is dat
klassiek netjes, maar onder BHT ~85 bit — de zwakte die eruit gehaald was door
naar 64-byte BLAKE3-XOF-knooppunten te gaan. En een SNARK met setup importeren
in een project waarvan het bestaansrecht is dat het géén SNARK nodig heeft, is
het doel opofferen aan het middel.

Hun stack is geoptimaliseerd voor een eigenschap die wij expres niet willen.
Dat is geen trots, dat is een rekensom. **Wat we wél overnemen: hun audit-lat
(drie audits, niet nul) en hun inzicht dat één boom veel gelijktijdige schrijvers
moet kunnen hebben.**

## 3c. Concurrentie: acceptatievenster in plaats van bewijstransformatie

Het probleem: twintig transacties in één slot willen allemaal notan minten of
besteden. Een bewijzer die wortel R zag, heeft bij binnenkomst een boom met
wortel R' — en mag alsnog niet botsen met negen andere schrijvers.

**Eerste poging (verworpen):** de change-log gebruiken om een bewijs van R naar
R' te *transformeren*, zoals Light doet. Twee defecten, beide gevonden door de
grondbewijstest over alle combinaties n×m vóórdat iets groen was:
1. het opgeslagen pad bevatte de wortel zelf, terwijl een wortel nooit een
   sibling is — élke update liep tegen de "geef niets terug"-tak;
2. een duplo-sibling (knood met zichzelf aan de rechterrand van een oneven
   niveau) stond als "links" genoteerd; werd die sibling later een écht buur-
   knoopje, dan verving ik de waarde maar niet de richting, en vouwde het bewijs
   verkeerd.

**Waarom die complexiteit er niet hoefde:** de accumulator is **append-only**.
Een blad dat in wortel R zat, zit over tien updates nog steeds in de boom —
bladeren worden hier nooit verwijderd of herschreven. Een bewijs tegen een
iets oudere wortel bewijst dus nog steeds iets warings. De enige vraag is of we
die wortel nog kunnen verantwoorden.

**De oplossing daarom (28 regels):** `Change { root, index }` in een ring van
laatst-geziene wortels (`DEFAULT_CHANGELOG = 64`), plus
- `accepts_root(root)` — huidige wortel, of een uit het venster;
- `verify_at(proof, index, leaf, root)` — vouw naar het aangereikte wortel én
  dat wortel valt binnen het venster.

Buiten het venster wordt geweigerd: niet omdat het bewijs onwaar zou zijn, maar
omdat we dan geen begrensd verhaal meer hebben. Falen boven gokken. Kosten:
73–85 ns.

**Les voor de methode:** de test die ik vóór de implementatie schreef (n×m
combinaties, inclusief de groei-overgangen 1→2, 3→4, 7→8) vond beide defecten
direct. Een tweede les: mijn eerste meting van dit venster zei 0,003 µs — gemeten
tegen de huidige wortel, die via de eerste toets meteen terugkeert. De echte
scan kost 25× meer. Meet het pad dat je bedoelt, niet het dichtstbijzijnde.

## 3d. Anonimiteitsmodel: menigte in de emmer (vastgelegd, met getal)

**Wat ik eerst dacht en waarom het niet deugt.** Als optie voor de bestedings-
laag stond erin: "begrenze sets, de hele set meesturen — eenvoudig, PQ-veilig".
Dat is twee keer onjuist. Grootte: 1.000 commitments à 64 byte is 64 KB tegen
een limiet van 4.096 — vijftien keer te groot. Erger: het model anoniemeert
niets. Wie zijn eigen opening meestuurt, geeft de verifieerder de mogelijkheid
te hashen en te kijken welke notitie het was. De anonimiteit verdwijnt op het
moment dat je hem bewijst.

**Wat wél kan met alleen hashes en PQ-handtekeningen.** Een besteding maakt
publiek: één nullifier (zonder geheim niet koppelbaar aan een commitment) en
één nieuwe commitment. Een waarnemer ziet dus alleen: in emmer (epoch, niveau)
is er één verbrand en één bijgekomen. Het aantal notan van dát niveau in dát
epoch bepaalt hoeveel kandidaten hij heeft om input aan output te koppelen.

**De eigenschap die dit bruikbaar maakt: de anonimiteitset kost nul extra
bytes.** Hij groeit met gebruik, niet met transactiegrootte. Vergelijk dat met
het set-model, waar een tiemaal zo grote set tienmaal zoveel bytes kostte.

| gemeten | waarde |
|---|---|
| bestedingsrecord (nullifier + commitment) | **128 byte** |
| + FN-DSA-512 handtekening | 937 byte |
| + ~200 byte instructie-/accountoverhead | **1.137 byte van 4.096** |
| tijd per besteding (20.000 records) | **4,85 µs** |
| kandidaten bij emmer van doel 100 | 101 (de +1 is de zojuist geminte output) |

**De knop is dus een gebruiksknop, en die is een productkeuze:** `DOELMENIGTE =
100` in `src/spend.rs`. Daaronder is een besteding bij benadering koppelbaar;
de code dwingt niets af, want de keten kan niet weten hoeveel opnames "genoeg"
zijn. Wat wél afgedwongen wordt: niveauvastheid per besteding, en dubbeluitgave.

**Twee grootheden, niet één.** Mijn eerste testbewering ("besteden houdt de
emmer gelijk groot") faalde, en kwam door mijn eigen redenering: een besteding
voegt wél een commitment toe (opnames +1) en haalt één besteedbare notitie weg
(live −1). Opnames zijn de kandidatenlijst voor "welke output is van mij", live
die voor "welke input verbrandde dit". Beide staan nu expliciet in de code
(`menigte` en `live`).

**Wat het model niet is:** geen sterke unlinkability. Het is menigte-anonimiteit
binnen (epoch, niveau) — sterker naarmate de emmer voller is, en met opzet
doorzichtbaar voor een houders van de auditsleutel via de audit-tag. Voor
stereografie zonder aannames is nog steeds een één-van-N-bewijs nodig, en dat
bestaat post-kwantum niet in geauditeerde vorm.

## 4. Afhankelijkheden en veiligheid

| crate | gebruik | status |
|---|---|---|
| `blake3` 1.8.6 | productie, enige primitief | officiële implementatie; NCC Group-audit (2020) |
| `sha2` 0.10.9 | dev-only (KAT-vectoren) | RustCrypto |
| `falcon-rs` 0.3.1 | dev-only (PQ-meting, FN-DSA/FIPS 206) | niet geauditeerd — alleen meten, niet tekenen |
| `dilithium-rs` 0.4.1 | dev-only (PQ-meting) | twee auditrondes, 2026-07-20 en 2026-08-31; alle twaalf bevindingen (incl. één HIGH: verkeerde OID in HashML-DSA; twee medium side-channel: `chknorm`, `make_hint`) staan als fixed |

**`ml-dsa` (RustCrypto) wordt niet gebruikt.** Die crate meldt zelf nooit
onafhankelijk geauditeerd te zijn en had CVE-2026-24850 / GHSA-5x2r-hc65-25f9
(verificatie aanvaardde handtekeningen met dubbele hint-indexen; getroffen
≥0.0.4, <0.1.0-rc.4). Er was dus niets te upgraden — er stond geen `ml-dsa` in
`Cargo.toml` noch in de lockfile. Bewust niet toegevoegd.

**Aanbevolen voor de signatuur-laag: hybride FN-DSA (Falcon) + Ed25519.** Een
fout in één van beide laat de transactie dan nog klassiek of post-kwantum
beveiligd; Falcon is de richting die Solana zelf noemt. Let op:
`ed25519-dalek` staat in de lokale cache in drie versies, waarvan de oude
RUSTSEC-advisories hebben — pinnen op een actuele versie is verplicht, niet
netjes.

---

## 5. Onderzoek, 2026-09-23 (invloed op het ontwerp)

- **PQ-ringhandtekeningen zijn actief onderzoek, geen productie.** DualRing-PRF
  (IACR eprint 2024/985), SPRING (lattices, 2025), hash-gebaseerde koppelbare
  PQ-ringhandtekeningen (MDPI Sensors 2025), PQ-drempel-ringhandtekeningen met
  efficiënte schaling naar grote ringen (ACM 2025). Er is wél een "één van de N"-
  primitief in onderzoek; er is géén geïmplementeerde, geauditeerde.
- **Light Protocol** bouwt en gebruikt concurrent Merkle trees en ZK Compression
  op Solana (productie voor cNFTs; Batched Merkle Trees en ZK V2 aangekondigd op
  Accelerate 2025). Voordat hier nog een boom geschreven wordt: vergelijken.
- **Token-2022 Confidential Transfers / Confidential Balances** lossen "bedrag
  verbergen" al op met Twisted ElGamal en on-chain geverifieerde ZK-bewijzen —
  klassiek, niet post-kwantum, maar bestaand en getest.

---

## 6. Openstaand

1. **Anonieme bestedingslaag** — **model vastgelegd** (zie 3d): menigte binnen
   emmer (epoch, niveau), record 128 byte, anonimiteit kost geen extra bytes.
   Optie "hele set meesturen" is vervallen: te groot én niet anoniem. Sterkere
   unlinkability wacht op PQ-ringhandtekeningen (research).
2. **Solana-integratie.** Nog geen code. Eerst de vergelijking met Light
   Protocol; daarna pas beslissen of eigen code nodig is.
3. **Hybride signatuur.** FN-DSA (Falcon) + Ed25519 — niet ML-DSA, want dat is
   niet wat Solana kiest. Met gepinde versies en vrachtaudit in de CI. Let op:
   `falcon-rs` is niet geauditeerd; voor productie is een auditeerde bron nodig
   of een eigen audit.
4. **Anonimiteitssetgrootte** — vastgelegd als `DOELMENIGTE = 100` per emmer
   (3d). Restende vraag is geen formaatvraag meer maar een vraag over verwachte
   opnames per epoch per niveau.
5. **Concurrentie** — **opgelost**: acceptatievenster, gemeten 73–85 ns (3c).

---

## 7. Werken met deze repo

```bash
cargo test                                 # 57 tests, inclusief aanvallen
cargo test -- --ignored --nocapture        # meetladder + PQ-ondergrens
cargo test --offline                       # zonder netwerk
```

Werkregels die deze sessie zijn opgelopen en helpen:
- **Geen handgeschreven cryptografie.** Geauditeerde crates, gepinde versies.
- **Aanvallen eerst laten falen, dan pas repareren.** Beide gaten in de boom
  waren onzichtbaar gebleven zonder die volgorde.
- **Metingen leven in de repo** (`src/bench.rs`, `#[ignore]`), niet in /tmp.
- **Een groene test is geen bewijs** — kijk waar het groen vandaan komt.
