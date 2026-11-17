# gravity-ledger

A Rust library for a ledger of fixed-denomination coins, with a BLAKE3 Merkle
accumulator and visible, Ed25519-signed spends.

**Status: research prototype, not audited. Do not use it for real value or in
production.**

## What it does

- Coins have 33 denomination levels, 2^0 to 2^32. Each coin enters an
  append-only Merkle accumulator as a leaf over its level and its commitment;
  the leaf is bound to its position. Every hash is BLAKE3 extended to 64 bytes,
  with a separate domain tag per context.
- A spend is visible and signed with Ed25519 (`verify_strict`).
- The ledger itself computes the input commitment, its leaf and the nullifier
  from the opening in the record, and checks membership of that leaf against a
  root in the acceptance window (the last 64 roots). The spender supplies the
  opening, the root, the proof and the output commitment; the output
  commitment is signed but not checked.
- A spend has exactly one input and one output at the same denomination level,
  so a spend cannot change value. `Shape::check` (a count per level for
  transactions with several inputs and outputs) exists but is not used by
  `Ledger::besteed`.
- A nullifier can be burned only once, so a coin cannot be spent twice.

## What it does not do

- There is no anonymity: the input and the output of a spend can be linked,
  and the public key and the denomination are visible.
- There are no post-quantum signatures in production code.
- `Ledger::mint` does not check who mints, nor the level or epoch of the
  coin: anyone who can call it creates value. A deployment must restrict
  minting.
- There is no Solana integration yet; no code for it exists.
- The root and the proof are not signed, so up to 64 variants of the same spend
  record exist, and anyone who has the record and the accumulator state at
  another root in the window can produce them. Never use a hash
  of a record as a transaction identifier; use the nullifier.

## Spend record (version 1)

Integers are u64 little-endian. A record is **276 + 64 × depth** bytes, with a
depth of at most 64.

| field | bytes |
|---|---|
| version (`0x01`) | 1 |
| scheme (`0x01` = Ed25519) | 1 |
| input index | 8 |
| denomination level (< 33) | 1 |
| input epoch | 8 |
| input secret | 32 |
| public key | 32 |
| root (in the acceptance window) | 64 |
| depth | 1 |
| proof | 64 × depth |
| output commitment | 64 |
| signature | 64 |

`dsh(tag, parts)` is BLAKE3 extended to 64 bytes over a domain-separated
encoding: a fixed prefix, the length of the tag, the tag, and each part
preceded by its length (lengths as u64 little-endian). `chain` is the 32-byte
identity of the chain (`ChainId`) that a ledger is created with
(`Ledger::new`); it goes into every commitment, nullifier and signed message,
so a record for one chain is not valid on another.

The nullifier is `N = dsh("GL/nullify/v1", [chain, input index, input
commitment])`. The signature covers `M = dsh("GL/besteding/v1", [chain,
version, scheme, N, level, output commitment])`; the root and the proof are
not part of it.

| leaves | depth | proof | record |
|---|---|---|---|
| 1,000 | 10 | 640 bytes | 916 bytes |
| 10,000 | 14 | 896 bytes | 1,172 bytes |
| 100,000 | 17 | 1,088 bytes | 1,364 bytes |

## Build and test

```bash
cargo test                                    # the test suite
cargo test --release -- --ignored --nocapture # the measurements
```

After the first download of the dependencies, `--offline` can be added to
either command. The measurement command also prints the post-quantum
signature measurements (FN-DSA and ML-DSA, from the dev-dependencies).

Rust version: the minimum supported version is 1.89 (`rust-version` in
Cargo.toml); the suite passes on Rust 1.89.0 and on stable 1.97.1. Older
versions have not been tested.

## Further reading

- `STATUS.md`: design, decisions and open points (in Dutch).
- `docs/`: measurements with machine and method, and the external review of
  10 November 2026 (both in Dutch).
- `SECURITY.md`: what the code protects and how to report a vulnerability.

## License

Apache-2.0; see `LICENSE`.
