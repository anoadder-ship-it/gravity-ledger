# gravity-ledger

Hash-only lidmaatschap en conservatie-equaliteit op Solana.

## Wat is gravity-ledger?

gravity-ledger is een Rust-basislaag voor Solana-programma's die hash-only bewijzen nodig heeft:
- **MerklePathProof**: bewijzen dat een leaf (noten-commitment) op positie i hoort bij de root van een accumulator
- **BinaryCommitment**: binary commitments per denomination (telling), compacte representatie van notenaantallen
- **ConservationProof**: conservatie-equaliteit bewijzen (hash-based inner-product claim), bewijs dat de totale waarde voor en na een transactie gelijk is

## Architectuur

gravity-ledger bestaat uit drie componenten:

1. **MerklePathProof** (`src/proof.rs`)
   - Bewijzen lidmaatschap van een leaf in een accumulator
   - Verify-functie bouwt de root op vanuit de leaf door te folden
   - Relaxed modus: acc mag overeenkomen met root op elk punt in de traversaal

2. **BinaryCommitment** (`src/proof.rs`)
   - Binary commitments per denomination (telling)
   - Compacte representatie: 1 byte denom + 8 bytes count + 32 bytes root = 41 bytes

3. **ConservationProof** (`src/proof.rs`)
   - Conservatie-equaliteit bewijzen (hash-based inner-product claim)
   - Bewijs dat de totale waarde voor en na een transactie gelijk is
   - Size: 32 bytes root + per_denom.len() * 5 * 8 bytes

## Build & Deploy

### Lokale build & test

```bash
cd ~/projects/gravity-ledger
rustc --edition 2021 --test src/lib.rs -o /tmp/gl_test
/tmp/gl_test
```

### Devnet deploy (nog niet geïmplementeerd)

```bash
cd ~/projects/gravity-ledger
export PATH="$HOME/projects/agave/bin:$PATH"
solana config set --url https://api.devnet.solana.com
solana program deploy target/deploy/gravity_ledger.so --keypair ~/.config/solana/id.json
```

## Roadmap

- **M0**: proof.rs bouwen en testen (AFGEROND, 29/29 tests groen)
- **M1**: binary_commitment.rs bouwen en testen
- **M2**: conservation_proof.rs bouwen en testen
- **M3**: E2E-matrix voor alle combinaties
- **M4**: Devnet deploy en smoke-test

## Kritieke gotchas

- **Solana CLI-config kan een verkeerde, gedeelde signer geven.** Altijd expliciet een keypair meegeven.
- **`solana program deploy` geeft standaard NUL upgrade-headroom.** Gebruik `solana program extend` vooraf.
- **SBPF-versie: moet arch v3 zijn.** Gebruik `cargo-build-sbf --arch v3`.

## License

Apache-2.0