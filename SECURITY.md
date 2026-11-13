# Security Policy

## Status

gravity-ledger is a research prototype. It has not been audited. Do not use it
to hold or move real value, and do not use it in production.

## What the code does and does not protect

What it does:

- A spend is visible and signed with Ed25519 (`verify_strict`).
- The ledger itself computes the input commitment, its leaf and the nullifier
  from the opening in the record, and checks membership of that leaf against a
  root in the acceptance window. The spender supplies the opening, the root,
  the proof and the output commitment; the output commitment is signed but not
  checked.
- A spend has exactly one input and one output at the same denomination level,
  so a spend cannot change value. `Shape::check` (a count per level for
  transactions with several inputs and outputs) exists but is not used by
  `Ledger::besteed`.

What it does not do:

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

The full list of open points is in `STATUS.md`, section 6.

## Reporting a vulnerability

Please report vulnerabilities privately: open the **Security** tab of this
repository and click **Report a vulnerability**.

Do **not** open a public issue for a vulnerability.
