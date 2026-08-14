# CEK Runtime — Grok build handoff

## What this is

Reference **CEK Host + Peer** implementation (Rust workspace) aligned with:

- Law: https://github.com/bitplorer/cek-framework
- Design: https://github.com/bitplorer/cek-runtime

## Verify first

```bash
cargo test --workspace
cargo run -p cek-cli -- demo
cargo run -p cek-cli -- vectors crates/cek-contract/vectors
./scripts/invariants.sh
node --experimental-strip-types --no-warnings \
  ports/cek-peer-ts/src/run-vectors.ts crates/cek-contract/vectors
```

Expected: **112** unit/property tests; **45** vectors PASS; TS Peer apply-only green; demo includes ui restore + kv.delete prior restore.

## Current maturity

**Stage C** — domain `ui.*` + snapshot reverse for UI **and** `kv.delete`. See MATURITY.md.

## Done this session (2026-08-14, kv.delete prior)

- `kv.delete` carries optional `prior` **on the Op** (same rule as `ui.morph` snapshot).
- Reverse is `kv.set` of that prior; missing prior → honest `non_reversible`.
- `inverse_kv` lives next to `inverse_ui` in the contract.

## Do next

1. WASM apply-only Peer (same `peer_result` vectors as TS).
2. Cap cryptographic signatures (optional Host policy).
3. llvm-cov HTML in CI.

## Never regress

- Cap refuse → zero mutate Ops
- BoundAsk only after verify
- Peer has no mint (Rust **and** TS)
- Once commit only after successful project
- Idempotency before once-ensure
- Landed-first reverse when receipt annotated
- Digests `cek1:` (FIPS SHA-256)
- Fail closed on unclear authority / store down / scope deny / attenuate widen
- Snapshot reverse only when snapshot is present (else mark non-reversible)

## Prompt starter

```
Continue CEK runtime from this workspace.
Read HANDOFF.md, IMPLEMENTATION.md, INVARIANTS.md first.
Run cargo test, vectors, invariants, and the TS peer runner before changing code.
Priority: Cap signatures or WASM Peer.
Do not violate Cap-only / Ops-as-data / Host-Peer split.
```
