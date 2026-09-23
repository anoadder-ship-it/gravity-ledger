# Security Policy

## Security Model

gravity-ledger gebruikt hash-only bewijzen voor lidmaatschap en conservatie-equaliteit:
- **MerklePathProof**: bewijzen lidmaatschap via Merkle-path verificatie
- **BinaryCommitment**: binary commitments per denomination
- **ConservationProof**: conservatie-equaliteit via hash-based inner-product claim

## Known Risks

- **secp256r1-precompile**: vereist WebAuthn-passkey-handtekening, nog niet getest tegen echte hardware
- **CPI-uitvoering**: execute's CPI-uitvoering is nog een no-op placeholder
- **Client-side integratie**: nog niet gestart, wacht op WebAuthn-fix

## Audit Status

- **proof.rs**: 29/29 tests groen, geen kwetsbaarheden gevonden
- **binary_commitment.rs**: nog niet geaudit
- **conservation_proof.rs**: nog niet geaudit

## Reporting a Vulnerability

Meld kwetsbaarheden via GitHub Issues.