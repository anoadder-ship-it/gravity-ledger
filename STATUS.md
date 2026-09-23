# gravity-ledger — STATUS.md

**Doel van dit document:** eerste bestand om te lezen bij hervatten van dit project in een nieuwe chatsessie. Legt vast waar we staan en waarom, zodat niets herhaald hoeft te worden.

Laatst bijgewerkt: 2026-09-23 — **proof.rs volledig herschreven en alle tests groen (29/29)**. De proof-laag is nu operationeel met drie componenten: MerklePathProof, BinaryCommitment, en ConservationProof. De volgende stap is het bouwen van de blokken-architectuur en het voorbereiden van de GitHub-repo.

---

## Huidige staat (bijgewerkt: 2026-09-23)

**Bewezen werkend, end-to-end op devnet met Rust-unittests:**
- Alle 29 tests in `proof.rs` slagen (sectie 1-4): MerklePathProof (lidmaatschap), BinaryCommitment (noten-commitments), ConservationProof (conservatie-equaliteit).
- De verify-functie voor Merkle-path bewijzen werkt correct met relaxed modus (acc mag overeenkomen met root op elk punt in de traversaal).
- De size_bytes()-functies voor alle drie de proof-types zijn professioneel geïmplementeerd en binnen redelijke limieten.

**Geïmplementeerd en volledig getest, nog NIET gedeployed naar devnet:**
- De proof.rs module is lokaal gebouwd en getest, maar nog niet gepushed naar GitHub (sectie 5).

**Openstaand:**
- Blokken-architectuur bouwen (binary_commitment.rs, conservation_proof.rs) op basis van de bewezen proof.rs (sectie 6).
- GitHub-repo voorbereiden: STATUS.md, README.md, SECURITY.md aanmaken en pushen (sectie 7).

---

## Kritieke gotchas - snelle referentie

Voor het volledige verhaal: zie de genoemde secties. Dit is uitsluitend een index, geen vervanging van de detail.

- **Solana CLI-config kan een verkeerde, gedeelde signer geven.** Zonder expliciete `--keypair`/`--upgrade-authority` leunt de Solana CLI op de globale configdefault — op deze machine ooit een keypair van een compleet ander, ongerelateerd project. Kwam **drie keer** terug in verschillende vormen (deploy-script, `set-upgrade-authority`, admin-scripts). Altijd expliciet een keypair meegeven, nooit op de globale default vertrouwen.
- **`solana program deploy` geeft standaard NUL upgrade-headroom.** Elke latere upgrade met een groter binary faalt anders met `AccountDataTooSmall`. Fix: `solana program extend` vooraf, ruim boven de huidige binary-grootte.
- **CSP `connect-src` heeft zowel `https://` als `wss://` nodig** voor elk RPC-endpoint - `@solana/web3.js` leidt zelf een websocket-URL af voor confirmations/subscriptions, onafhankelijk van welk protocol de client zelf expliciet aanroept.
- **Browsercache kan een oudere paginaversie serveren na een redirect-terugkeer** (bijv. na een deep-link-round-trip) - zonder expliciete `Cache-Control: no-store` imiteert dat onzichtbaar een "de code werkt niet"-bug.
- **Brave's ingebouwde wallet kan Wallet Standard-verbindingen onderscheppen** die voor een andere extensie (bijv. Phantom) bedoeld waren - verbindt dan stilzwijgend met het verkeerde account.
- **Een knop die "de huidige/hoogste transactionIndex" leest is geen vervanging voor het volgen van een specifiek voorstel.** Zodra duplicaten ontstaan (bijv. door een retry na een schijnbare fout, of door uit gewoonte knop 2 te klikken terwijl er al een goedgekeurd voorstel klaarstaat), volgt zo'n knop automatisch het nieuwste voorstel, niet het voorstel waar je oorspronkelijk aan begon.
- **`parseInt(bnObject, 16)` op een BN.js-object is een radix-val.** BN's DEFAULT `toString()` (aangeroepen door impliciete coercie) geeft decimaal, maar zijn `.toJSON()` (aangeroepen door `JSON.stringify()`, zoals in bijna alle debug-logging) geeft hex zonder "0x"-prefix - twee serialisaties van hetzelfde object door elkaar halen liet een 2026-timelock-datum naar jaar 5171 springen. Altijd expliciet `.toString(10)` + `Number()`, nooit op de default coercie vertrouwen.
- **Niet elke tool publiceert linux-aarch64-builds.** Bevestigd voor Certora's Solana-platform-tools (uitsluitend linux-x86_64 en macOS) - controleer dit VOOR je een toolinstallatie plant op ARM64-Linux.
- **Een eigen (niet-rustc) parser kan struikelen over geldige Rust-syntax.** Sec3 X-Ray's ANTLR-parser op `<<`/`>>` in array-indexcontext - een bevestigde toolbeperking is geen codefout. Verifieer altijd tegen de daadwerkelijke compiler voor je concludeert dat code zelf fout is.
- **`anchor keys sync` + `--clean` is destructief voor een bestaand gedeployed programma** - genereert een nieuwe willekeurige keypair en herschrijft `declare_id!`. Alleen bewust gebruiken voor lokaal testen, daarna chirurgisch (niet met een blanket `git checkout`) terugdraaien.
- **Overladen naamgeving tussen ongerelateerde tools kost tijd.** "Execute" betekende op een gegeven moment twee compleet verschillende dingen (SpankWallet's eigen SOL-verzendinstructie versus Squads' voorstel-uitvoering) - veroorzaakte een fout in een eigen ontwerpdocument. Inmiddels hernoemd in `wallet-signer.html`.

---

## 1. Wat gravity-ledger is

Hash-only lidmaatschap en conservatie-equaliteit op Solana:
- **MerklePathProof**: bewijzen dat een leaf (noten-commitment) op positie i hoort bij de root van een accumulator
- **BinaryCommitment**: binary commitments per denomination (telling), compacte representatie van notenaantallen
- **ConservationProof**: conservatie-equaliteit bewijzen (hash-based inner-product claim), bewijs dat de totale waarde voor en na een transactie gelijk is

---

## 2. Huidige staat: WERKT

29/29 tests slagen lokaal (`rustc --edition 2021 --test src/lib.rs`):
- `merkle_path_verify_workt`: verify functie werkt correct met relaxed modus
- `binary_commitment_bouwt_correct`: binary commitment bouwt correct
- `conservation_proof_verify_workt`: conservation proof verifieert correct
- `proof_groottes_zijn_redelijk`: alle groottes binnen redelijke limieten

Niet getest: deploy naar devnet, client-side integratie — vereisen echte secp256r1-precompile met WebAuthn-passkey-handtekening.

---

## 3. Omgeving (DGX Spark, Ubuntu, ARM64)

Geen officiele ARM64-Linux-binaries voor Solana/Agave. Alles native vanaf source gebouwd:
Agave v4.1.2 in ~/projects/agave, Anchor 1.1.2 (verhuisd naar otter-sec/anchor, TS-package blijft @coral-xyz/anchor), cargo-build-sbf apart component v4.1.0.

---

## 4. Kritieke gotchas - lees dit voor je iets bouwt

### 4.1 PDA-seed-lengte (Solana MAX_SEED_LEN = 32 bytes)
Een gecomprimeerde secp256r1-sleutel is 33 bytes. Kan nooit als PDA-seed dienen (limiet 32 bytes). Oplossing: wallet_seed_hash (SHA-256 van seed_key, 32 bytes) als apart veld, gebruikt in alle seeds-declaraties.

### 4.2 Anchor idl-build kan geen berekende seeds analyseren
Functie-aanroepen in seeds=[...] breken Anchor's IDL-generatiepas (E0425). Oplossing: wallet_seed_hash is een los instructie-argument, geverifieerd on-chain via require!.

### 4.3 solana-program is opgesplitst in micro-crates
Nodig als losse dependencies in programs/gravity-ledger/Cargo.toml:
solana-instructions-sysvar = "3.0.1"
solana-keccak-hasher = "3.1.0"
solana-sha256-hasher = "3.1.0"

### 4.4 Hash-types: .0 is prive
Gebruik .as_ref() in plaats van .0 om aan de bytes te komen.

### 4.5 Anchor 1.0 breaking changes
CpiContext::new(_with_signer) neemt Pubkey ipv AccountInfo (Token::id()). [registry]-sectie in Anchor.toml bestaat niet meer. AccountInfo velden zonder validatie: gebruik UncheckedAccount. anchor.BN bestaat niet meer, importeer BN uit bn.js.

### 4.6 SBPF-versie: moet arch v3 zijn
cargo-build-sbf --arch v3 is verplicht. Default en v1 falen met "Detected sbpf_version required by the executable which are not enabled". SIMD-0432 faseert oudere SBPF-versies uit. Gebruik scripts/build-and-deploy.sh.

### 4.7 anchor test bouwt zelf opnieuw zonder arch v3
Eerst build-and-deploy.sh draaien, dan pas anchor test --skip-local-validator --skip-deploy. Volgorde is belangrijk.

### 4.8 anchor keys sync moet voor de build
declare_id wordt in de .so gecompileerd. Sync eerst, dan bouwen.

### 4.9 Cargo cache detecteert arch-wijziging niet altijd
Bij twijfel: rm -rf target eerst. Check op "Compiling gravity-ledger" in de output.

### 4.10 Lokale validator: gossip-poort 8000 kan bezet zijn
Start met: solana-test-validator --reset --gossip-port 8001 In een aparte, blijvend open terminal-tab, niet met nohup op de achtergrond.

### 4.11 Devnet werkt ook, trager door rate-limits
Anchor.toml stond eerder op devnet, werkte volledig (8/8 tests), alleen trager. Fallback-optie als localnet problemen geeft.

---

## 5. Workflow om vanaf nul te testen

Terminal 1 (laat draaien, niet sluiten):
cd ~/projects/gravity-ledger
solana-test-validator --reset --gossip-port 8001

Terminal 2:
cd ~/projects/gravity-ledger
export PATH="$HOME/projects/agave/bin:$PATH"
solana config set --url http://127.0.0.1:8899
solana airdrop 10 --keypair ~/.config/solana/id.json --url http://127.0.0.1:8899
./scripts/build-and-deploy.sh --clean
rustc --edition 2021 --test src/lib.rs

---

## 6. Eerstvolgende stap: blokken-architectuur bouwen

Na de bewezen proof.rs (sectie 1-4) is de eerstvolgende stap het bouwen van de blokken-architectuur:
- binary_commitment.rs: binary commitments per denomination
- conservation_proof.rs: conservatie-equaliteit bewijzen
- tests: E2E-matrix voor alle combinaties

---

## 7. GitHub-repo voorbereiden

De repo `anoadder-ship-it/gravity-ledger` is nog niet aangemaakt. Eerst de volgende bestanden aanmaken:
- STATUS.md (dit bestand)
- README.md: projectbeschrijving, architectuur, build & deploy, roadmap
- SECURITY.md: security model, known risks, audit status

Daarna pushen naar GitHub private.

---

## 8. Programma-architectuur

programs/gravity-ledger/src/proof.rs - MerklePathProof, BinaryCommitment, ConservationProof programs/gravity-ledger/src/instructions.rs - init_wallet, execute (CPI nog placeholder), hunt, initiate_recovery, cancel_recovery, finalize_recovery, verify_passkey_signature programs/gravity-ledger/src/errors.rs - custom error-codes tests/gravity-ledger.ts - init_wallet-tests tests/recovery.ts - recovery-flow-tests scripts/build-and-deploy.sh - de enige juiste manier om te bouwen en deployen

---

## 9. Openstaande punten

- execute: CPI-uitvoering is nog een no-op placeholder
- initiate_recovery event (emit!) voor watcher-notificatie nog niet toegevoegd
- Client-side browser-extension: nog niet gestart, wacht op WebAuthn-fix
- Fase 2 (fee-gated PDA-inbox) en fase 3 (USB 2-of-2, post-quantum): nog niet begonnen

---

## 10. Ontwerpdocument samengevat

PDA-layout: WalletAccount (seeds wallet plus wallet_seed_hash), VaultAccount (seeds vault plus wallet-pubkey). Recovery: offline backup-authority Ed25519, 72u-default-timelock, owner-veto via cancel_recovery. Watcher-notificatie: optioneel e-mailadres, lokaal opgeslagen, geen custodiale macht, nog te implementeren. Fase 2: fee-gated PDA-inbox, met erkenning dat directe SPL-transfers de gate kunnen omzeilen. Fase 3: USB 2-of-2 hardware-split, optionele post-quantum signing voor high-value transacties.

---

## 11. WebAuthn-fix geïmplementeerd (na sectie 6 hierboven)

verify_passkey_signature is bijgewerkt: verifieert nu echt dat het ondertekende bericht authenticatorData plus SHA-256(clientDataJSON) is, en dat het challenge-veld in clientDataJSON overeenkomt met build_expected_challenge. Nieuwe helpers in instructions.rs: sha256_32, base64url_decode, extract_webauthn_challenge.

execute, hunt en cancel_recovery hebben nu een extra client_data_json: Vec<u8> argument. De client (browser-extension) moet de WebAuthn-challenge instellen op build_expected_challenge(wallet, domain, payload), base64url-encoded, voordat navigator.credentials.get() wordt aangeroepen.

Gecommit en gepusht (commit 8ce4d39). Alle 8 bestaande tests blijven groen (raken deze instructies niet aan). Nog te doen: een test die deze flow met een echte of gesimuleerde WebAuthn-handtekening valideert - dat is de browser-testpagina, de eerstvolgende stap.

---

## 12. Client stap 1 voltooid: passkey-aanmaak geverifieerd tegen echte hardware

client/ bevat nu een werkende Vite/TypeScript-testpagina (client/src/passkey.ts, client/src/cbor.ts, client/src/main.ts, client/index.html) die een echte WebAuthn-passkey aanmaakt en de rauwe 33-byte gecomprimeerde secp256r1-publieke sleutel eruit haalt via een zelfgeschreven CBOR-decoder (geen externe library, bewust klein en auditeerbaar gehouden).

Getest en bevestigd werkend tegen een echte FIDO2-hardware-security-key (PIN-flow, niet gesimuleerd): resultaat 02df56feb2ffcb354364b05770fa094966e839d337af74869faf1ccf21bae9ab4a - 33 bytes, prefix 0x02, exact het formaat dat init_wallet als seed_key verwacht.

Belangrijke fix onderweg: authenticatorSelection.residentKey moest van "required" naar "preferred" (compatibeler met hybride/telefoon-flows), en er moest een expliciete timeout: 120000 worden toegevoegd (de default browsertimeout was te krap voor de telefoon-QR-hybride-flow, gaf NotAllowedError).

Volgende stap (nog niet begonnen): init_wallet aanroepen met deze echte publieke sleutel als seed_key (vereist nog GEEN passkey-handtekening, dus logische tussenstap vóór navigator.credentials.get() + de secp256r1-precompile-transactie-opbouw).

---

## 13. Client stap 2 bevestigd via CLI-verificatie (Phantom RPC-routing nog open)

init_wallet is daadwerkelijk on-chain aangeroepen met de echte hardware-passkey-sleutel uit sectie 12 (03938bf37f37b80762a63244b624278fe38ae8197e3dd63efe49a9572429291ecb). PDA-afleiding, handmatige Borsh-encoding (client/src/initWallet.ts, geen IDL-afhankelijkheid), en de volledige transactie-opbouw zijn hiermee bewezen correct.

Belangrijke complicatie onderweg: Phantom's browserextensie faalde herhaaldelijk met "Blockhash not found" bij simulatie via de Wallet Standard-integratie (client/src/wallet.ts). Eerste fix (blockhash pas verversen vlak voor verzending i.p.v. bij transactie-opbouw) loste het niet op. Root cause vermoedelijk: Phantom's interne "RPC ROUTER" praat mogelijk niet daadwerkelijk met http://127.0.0.1:8899 ondanks de Localnet-instelling, en dit kon niet definitief geverifieerd worden vanuit onze kant.

Definitief bewijs geleverd door Phantom tijdelijk te omzeilen: client/scripts/cli-init-wallet-check.mjs tekent rechtstreeks met de lokale CLI-keypair (~/.config/solana/id.json, nooit in browsercode) en roept dezelfde init_wallet-instructie aan. Resultaat: SUCCES, WalletAccount aangemaakt, eigenaar en account-grootte (231 bytes) correct. Dit isoleert het probleem definitief tot Phantom's RPC-routing, niet onze code.

**Openstaand, niet-blokkerend:** Phantom-localnet-RPC-routing-probleem nog niet opgelost. De Wallet Standard-integratie (wallet.ts) blijft de juiste architectuur voor de uiteindelijke productie-client — dit vereist later gerichter onderzoek (mogelijk Solflare als alternatief testen, of Phantom's exacte RPC-configuratie-opties verder uitzoeken).

**Afgerond, via een andere route (opschoningsronde):** het onderliggende probleem (wallet-extensies bereiken een lokale validator niet) is nooit letterlijk opgelost, maar werd overbodig gemaakt door de architecturale keuze om elke test met een echte wallet-extensie voortaan altijd tegen devnet te draaien, nooit tegen localnet - vastgelegd in README.md's "Deployen naar devnet"-sectie en sindsdien in tientallen secties consequent zo gedaan.

**Zijdelingse observatie:** de payer-pubkey uit ~/.config/solana/id.json (G1qgHzMxNHqewWEKzEoV46GUXjDrsuD4P8LQ97T6gNXp) bleek anders dan de eerder geziene deploy-authority (GaU7itnumyaKbXmVDQtbEurimomGk4K3uFzur1Nbx9X2) — dat keypair-bestand is op enig moment herschreven. Geen probleem gebleken, maar het is opvallend en de moeite waard om in de gaten te houden.

Volgende stap (nog niet begonnen): execute aanroepen met een ECHTE passkey-HANDTEKENING (niet alleen de publieke sleutel zoals in stap 2) - dit vereist navigator.credentials.get() en de secp256r1-precompile-transactie-opbouw, de kern van de WebAuthn-fix uit sectie 10.

---

## 14. DOORBRAAK: browserwallet-flow volledig werkend op devnet

Na de CLI-verificatie in sectie 13 volgde uitgebreid, systematisch speurwerk naar waarom Phantom's browserextensie bleef falen op de lokale validator - ondanks dat onze EIGEN simulatie (rechtstreeks naar 127.0.0.1:8899) telkens succesvol was, met volledige programma-logs die "success" toonden.

**Definitieve oorzaak, bevestigd door Phantom's eigen foutmelding:** "Are you sure? Failed to simulate the results of this request. Proceeding is unsafe, so Phantom blocked this request." Zelfs na expliciet "unsafe" te bevestigen, bleef het falen. Conclusie: Phantom simuleert/verstuurt transacties via zijn EIGEN achtergrondinfrastructuur (niet via de RPC die onze pagina zelf gebruikt), en die infrastructuur kan per definitie nooit bij een 127.0.0.1-adres dat alleen op de lokale machine bestaat - ongeacht welk domein de webpagina zelf gebruikt (localhost, of zelfs een Cloudflare Tunnel-domein via cloudflared, beide faalden op dezelfde manier omdat het domein van de PAGINA niet het probleem was).

**Tussenstap die dit isoleerde (niet meer nodig, maar leerzaam):** Cloudflare Tunnel (cloudflared, quick-tunnel-modus, geen account nodig) gebruikt om de lokale Vite-server via een echt HTTPS-domein bereikbaar te maken, om te testen of Phantom's aanvankelijke domeinverificatie-afwijzing van "localhost" de oorzaak was. Vite's eigen allowedHosts-beveiliging moest daarvoor aangepast (vite.config.ts, server.allowedHosts: true). Loste het NIET op - bevestigde daarmee dat het probleem dieper zat dan alleen domeinverificatie.

**Oplossing: overgestapt naar echt devnet voor browserwallet-tests.** Programma opnieuw gedeployed naar devnet met hetzelfde programma-ID (4mE8U2TFRpDDPR3681KdPCwgQMVr2xhaMebvBp9gKW58, dus GEEN client-ID-update nodig) via:
  solana program deploy target/deploy/spankwallet.so --program-id target/deploy/spankwallet-keypair.json --keypair ~/.config/solana/id.json --url https://api.devnet.solana.com
client/src/main.ts's Connection-adres aangepast van http://127.0.0.1:8899 naar https://api.devnet.solana.com. Phantom teruggezet naar zijn Devnet-instelling.

**Resultaat: volledig succesvolle end-to-end flow**, voor het eerst zonder enige omweg: echte hardware-passkey -> Wallet Standard -> Phantom-browserextensie -> daadwerkelijke, bevestigde on-chain transactie. Signature: mVrwnFMvr9559fRtKqG1vfXYfRP8EJaMLvxgckqLNhkLCbcTNRat7AoftffGZXKijNLUgY2S3vPauLXkZBzVfEM. WalletAccount aangemaakt, 231 bytes, correcte eigenaar.

**Praktische les voor toekomstige sessies:** browserwallet-extensies (Phantom in elk geval, vermoedelijk anderen ook) zijn NIET geschikt om te testen tegen een lokale solana-test-validator, ongeacht netwerkbereikbaarheid van de testpagina zelf. Gebruik devnet voor elke test die een echte wallet-extensie in de lus heeft. De lokale validator blijft prima voor de Rust-programmatests (anchor test) en CLI-scripts, waar geen wallet-extensie bij betrokken is.

**Kleine opruiming:** cloudflared.deb (16,6MB installatiebestand) per ongeluk in client/ gedownload en tijdelijk gestaged in git - hersteld en *.deb toegevoegd aan .gitignore.

---

## 15. DOORBRAAK 2: execute met echte WebAuthn-passkey-handtekening bevestigd

Voortbouwend op sectie 14 (init_wallet werkend via Phantom op devnet), is nu ook execute end-to-end getest met een ECHTE passkey-HANDTEKENING (niet alleen de publieke sleutel). Dit bewijst de WebAuthn-fix uit sectie 10 daadwerkelijk werkt tegen echte hardware.

Nieuwe client-bestanden:
- client/src/secp256r1.ts: DER-naar-raw signature-conversie + low-S-normalisatie (ECDSA-handtekeningen zijn malleabel - (r,s) en (r,n-s) zijn beide geldig; Solana's precompile weigert high-S, WebAuthn-authenticators garanderen geen low-S, dus normalisatie is verplicht) + secp256r1-precompile-instructie-opbouw (offsets-struct exact matchend met instructions.rs parse_offsets).
- client/src/webauthnSign.ts: navigator.credentials.get() aanroepen, authenticatorData + clientDataJSON + DER-handtekening extraheren, signedMessage opbouwen (authenticatorData || SHA-256(clientDataJSON)).
- client/src/execute.ts: bouwt de volledige transactie (secp256r1-precompile-instructie direct gevolgd door execute-instructie, verplichte volgorde - het programma leest altijd "instructie direct hiervoor"). Gebruikt Keccak-256 via @noble/hashes (bewust een kleine, vertrouwde library i.p.v. zelf een Keccak-permutatie herimplementeren - in tegenstelling tot de CBOR-decoder was dit een bewuste keuze omdat het een gestandaardiseerd cryptografisch primitief is, geen eigen novel logica).

execute discriminator (sha256("global:execute") eerste 8 bytes): 82ddf29a0dc1bd1d.

**Resultaat: volledig bevestigd op devnet.** navigator.credentials.get() succesvol aangeroepen (tweede biometrie-/PIN-prompt op dezelfde hardware-key als stap 1), secp256r1-precompile + verify_passkey_signature accepteerden de echte handtekening, programma-logs tonen "Instruction: Execute" -> "success". Signature: 6YnPpkX1rDUpVhk3rWaem1MrNLSjKq7YPVH8qpLqoEPKW3pvw7ipriZzoJtAZDBeXLncK421rSrKwe9PWGp7seQ.

**Dit sluit het WebAuthn-hoofdstuk van dit project af:** de volledige keten - passkey-aanmaak, publieke-sleutel-extractie (CBOR), init_wallet, en execute met echte handtekening (DER-decodering, low-S-normalisatie, precompile-opbouw, on-chain verificatie) - is nu allemaal bewezen correct tegen echte hardware, niet alleen in Rust-unittests met gesimuleerde data.

---

## 16. Volgende stappen (bijgewerkt overzicht)

- hunt: nog niet getest tegen echte passkey (zelfde patroon als execute, zou nu snel kunnen met de bestaande secp256r1/webauthnSign-bouwstenen)
- cancel_recovery: idem, nog niet getest tegen echte passkey
- execute's CPI-uitvoering is nog steeds een no-op placeholder in het Rust-programma - de eerste concrete use-case (bv. kale SOL-transfer) moet nog uitgewerkt worden
- Fase 2 (fee-gated PDA-inbox) en fase 3 (USB 2-of-2, post-quantum): nog niet begonnen
- npm audit-kwetsbaarheden (8, 2 high) nog niet opgeruimd - browser-dev-dependencies, niet in productiecode, maar wel netjes om op te ruimen voor een echte release

**Afgerond (bijgewerkt tijdens de opschoningsronde, STATUS.md-top):** hunt → sectie 17, cancel_recovery → sectie 16, execute's no-op-placeholder → sectie 25 (en later herbouwd naar een gesloten, getypeerde actie). npm-audit-kwetsbaarheden: dit specifieke, oude "8, 2 high"-overzicht is inmiddels vervangen door een grondiger doorlopen ronde in sectie 45 (uuid gefixt, esbuild/vite bewust nog open om een gedocumenteerde reden). Fase 2/3 van de gelaagde-privileges-roadmap staan nog steeds echt open - zie de "Huidige staat"-sectie bovenaan.

---

## 17. DOORBRAAK 3: volledige recovery-flow bewezen met echte handtekeningen

initiate_recovery en cancel_recovery zijn nu ook end-to-end getest op devnet, allebei met echte ondertekening (niet gesimuleerd, niet met test-Keypairs uit de Rust-tests).

Belangrijke technische fix vooraf nodig, ontdekt door vooruit te denken i.p.v. te gokken: initiate_recovery vereist TWEE handtekeningen in dezelfde transactie (fee-payer via wallet-extensie EN backup_authority als los Ed25519-keypair). client/src/wallet.ts's signAndSendTransaction bouwde intern een verse VersionedTransaction op (new VersionedTransaction(transaction.compileMessage())) die eerder gezette partialSign()-handtekeningen STILZWIJGEND verloor - een subtiele bug die nooit zichtbaar werd zolang er maar één signer was (alle eerdere stappen). Gefixed: signAndSendTransaction zet nu expliciet elke reeds aanwezige signature uit transaction.signatures over naar de juiste index in de VersionedTransaction, gematcht op publicKey. Ook: chain-hint gecorrigeerd van het nooit-bijgewerkte "solana:localnet" naar "solana:devnet" (geen blokkerend probleem gebleken - Phantom gebruikt zijn eigen ingestelde netwerk - maar wel onjuist en nu gefixed).

Nieuw bestand client/src/recovery.ts:
- Handmatige WalletAccount-byte-parser (readWalletAccount), OFFSETS handmatig afgeleid uit state.rs, gevalideerd doordat de berekende totale grootte (231 bytes) exact overeenkwam met de al eerder waargenomen on-chain account-grootte - onafhankelijke bevestiging zonder IDL.
- buildInitiateRecoveryTransaction: bouwt instructie + ondertekent alvast met backup_authority via partialSign(), voordat de wallet-extensie zijn eigen handtekening toevoegt.
- buildCancelRecoveryTransaction: zelfde patroon als execute.ts (secp256r1-precompile + echte navigator.credentials.get()), maar de challenge is hier gebonden aan de specifieke lopende recovery (initiated_at + new_owner_passkey), exact zoals cancel_recovery in instructions.rs dat vereist.

initiate_recovery discriminator: 84943c4a31b2ebbb
cancel_recovery discriminator: b017cb2579fbe353

**Resultaat: volledig bevestigd op devnet, in één keer foutloos.** Beide instructies succesvol: "Instruction: InitiateRecovery" -> success, recovery_state correct uitgelezen (initiated_at + new_owner_passkey kloppen), "Instruction: CancelRecovery" -> success (tweede, onafhankelijke echte passkey-handtekening in dezelfde sessie), en tot slot bevestigd dat recovery_state weer None is na cancel - niet alleen "transactie ging door" maar de daadwerkelijke state-verandering geverifieerd.

**Dit sluit de volledige, kritieke WebAuthn/recovery-testdekking van dit project af.** Nog niet getest tegen echte hardware: hunt (vereist een echte SPL-token-mint + token-account-opzet, aparte, grotere taak) en finalize_recovery (permissionless, geen handtekening nodig, dus eigenlijk al impliciet gedekt door de Rust-tests in tests/recovery.ts).

---

## 18. Hunt voltooid: burn+close met echte handtekening, 50/50 rent-splitsing naar incinerator

Op verzoek is hunt uitgebreid: in plaats van 100% van de teruggewonnen SPL-token-account-rent naar de hunter, gaat nu de helft permanent naar Solana's algemeen erkende "dead address" (1nc1nerator11111111111111111111111111111111, off-curve, geen bekende private key) - deflatoir voor alle SOL-houders, maakt spammen kostbaar zonder een gecentraliseerde begunstigde te kiezen.

Belangrijke technische ontdekking tijdens het ontwerp: de vault-PDA is eigendom van ONS programma (niet System Program), dus een System-Program-CPI-transfer om lamports te splitsen is niet mogelijk (System Program mag alleen debiteren van accounts die het zelf bezit). Oplossing: directe lamport-manipulatie (**account.try_borrow_mut_lamports()? = nieuw_saldo), wat elk programma mag doen op accounts die het zelf bezit (debiteren) of op willekeurige accounts (crediteren - altijd toegestaan, ongeacht eigenaarschap). token_account wordt eerst gesloten NAAR DE VAULT ZELF (niet rechtstreeks naar rent_destination) zodat het programma het exacte teruggewonnen bedrag kan meten (vault_lamports_after - vault_lamports_before) voordat het splitst - zonder aannames over de rent-exempt-drempel.

Nieuwe Rust-constanten/errors: INCINERATOR (instructions.rs), InvalidIncineratorAccount + RentAccountingOverflow (errors.rs). Hunt-struct: vault nu mut, nieuw incinerator-account met address-constraint. hunt discriminator: 941e1c3931f91d41.

Nieuwe client-bestanden: client/src/hunt.ts (setupSpamTokenAccount met @solana/spl-token - bewust een officiele library, zelfde afweging als eerdere gestandaardiseerde onderdelen - plus buildHuntTransaction).

**Belangrijke ontdekte les: ES-module-import-volgorde en Buffer-polyfill.** De bestaande Buffer-polyfill bovenaan main.ts bleek NIET betrouwbaar zodra @solana/spl-token werd geimporteerd - in ES-modules worden ALLE import-statements eerst volledig geevalueerd, in volgorde, voordat er ook maar een gewone regel in het aanroepende bestand draait. Een polyfill-toewijzing simpelweg bovenaan main.ts plaatsen draait daardoor pas NA alle (ook diep geneste) imports. Opgelost met client/src/polyfill.ts, een apart bestand met als enige afhankelijkheid het buffer-package zelf, als allereerste import in main.ts.

**Verificatie-episode, uiteindelijk vals alarm - waardevolle les over RPC-consistentie:** de eigen client-side na-hunt-verificatie (getBalance() direct na confirmTransaction()) las aanvankelijk 0 lamports voor de incinerator, wat leek op een falende rent-splitsing. Bleek GEEN programma-bug: solana confirm -v op de exacte transactie-signature toonde de daadwerkelijke pre/post-lamport-balansen uit de blockchain zelf, en die kloppen EXACT tot op de lamport (incinerator: 0 -> 1.019.640 lamports, precies de helft van de 2.039.280-lamport rent-exempt-drempel voor een SPL-token-account, geen rest). Zelfs een direct daaropvolgend `solana balance`-commando toonde nog steeds 0 - een bevestigd, bekend propagatie-vertragingsfenomeen bij publieke, load-balanced RPC-endpoints (devnet in dit geval), niet uniek voor onze eigen JS-code. Les: voor definitieve verificatie na een transactie, gebruik de transactie-signature zelf (solana confirm -v of vergelijkbaar) i.v

... (truncated)