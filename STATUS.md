# gravity-ledger — STATUS.md

**Doel:** eerste bestand bij hervatten. Legt vast waar we staan, wat bewezen is,
en welke keuzes waarom zijn gemaakt — zodat niets herhaald hoeft te worden.

Laatst bijgewerkt: 2026-09-23. **46/46 tests groen; alle prestatiegetallen
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
Gemeten: ML-DSA-44 signatuur = 2.420 byte, terwijl het hele
lidmaatschapsbewijs 1.105 byte is. De handtekening bepaalt de ondergrens; per
noot signeren vermenigvuldigt die.

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

Caveats, expliciet: één run per meting, release-build, één kern, 30 iteraties
voor PQ. ML-DSA-tekentijden zijn variabel door afwijzingsbemonstering (264 µs
voor level 87, sneller dan 65) — dat is een eigenschap van het algoritme, geen
meetfout, en het telt mee bij harde bloktijd-deadlines.

---

## 4. Afhankelijkheden en veiligheid

| crate | gebruik | status |
|---|---|---|
| `blake3` 1.8.6 | productie, enige primitief | officiële implementatie; NCC Group-audit (2020) |
| `sha2` 0.10.9 | dev-only (KAT-vectoren) | RustCrypto |
| `dilithium-rs` 0.4.1 | dev-only (PQ-meting) | twee auditrondes, 2026-07-20 en 2026-08-31; alle twaalf bevindingen (incl. één HIGH: verkeerde OID in HashML-DSA; twee medium side-channel: `chknorm`, `make_hint`) staan als fixed |

**`ml-dsa` (RustCrypto) wordt niet gebruikt.** Die crate meldt zelf nooit
onafhankelijk geauditeerd te zijn en had CVE-2026-24850 / GHSA-5x2r-hc65-25f9
(verificatie aanvaardde handtekeningen met dubbele hint-indexen; getroffen
≥0.0.4, <0.1.0-rc.4). Er was dus niets te upgraden — er stond geen `ml-dsa` in
`Cargo.toml` noch in de lockfile. Bewust niet toegevoegd.

**Aanbevolen voor de signatuur-laag: hybride ML-DSA + Ed25519.** Een fout in één
van beide laat de transactie dan nog klassiek of post-kwantum beveiligd. Let op:
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

1. **Anonieme bestedingslaag.** Het enige echte ontwerpprobleem. Opties:
   (a) begrenze sets met meesturen van de hele set — eenvoudig, PQ-veilig,
   lineair in setgrootte; (b) wachten op PQ-ringhandtekeningen (research);
   (c) een ander model voor anonimiteit kiezen. Geen SNARK: die breekt de
   post-kwantum-eigenschap die dit project zijn bestaansrecht geeft.
2. **Solana-integratie.** Nog geen code. Eerst de vergelijking met Light
   Protocol; daarna pas beslissen of eigen code nodig is.
3. **Hybride signatuur.** ML-DSA + Ed25519, met gepinde versies en een
   vrachtaudit (cargo-audit / cargo-deny) in de CI.
4. **Anonimiteitssetgrootte vastleggen.** Niet "zo groot mogelijk" maar een
   getal per bucket; de kosten zijn gemeten (650 → 1.105 byte).

---

## 7. Werken met deze repo

```bash
cargo test                                 # 46 tests, inclusief aanvallen
cargo test -- --ignored --nocapture        # meetladder + PQ-ondergrens
cargo test --offline                       # zonder netwerk
```

Werkregels die deze sessie zijn opgelopen en helpen:
- **Geen handgeschreven cryptografie.** Geauditeerde crates, gepinde versies.
- **Aanvallen eerst laten falen, dan pas repareren.** Beide gaten in de boom
  waren onzichtbaar gebleven zonder die volgorde.
- **Metingen leven in de repo** (`src/bench.rs`, `#[ignore]`), niet in /tmp.
- **Een groene test is geen bewijs** — kijk waar het groen vandaan komt.
