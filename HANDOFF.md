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
bash scripts/run-wasm-peer.sh
```

Expected: **114** unit/property tests; **45** vectors PASS; TS + WASM Peers apply-only green.

## Current maturity

**Stage C met** for domain pack + snapshot reverse. **Stage D started** — TS and WASM apply-only Peers.

## Done this session (2026-08-14, WASM Peer)

- `cek-peer-wasm`: JSON apply surface wrapping the Rust Peer. **No mint.**
- `wasm32-unknown-unknown` cdylib + Node runner on the same `peer_result` vectors.
- World snapshots (`kv` / `ui`) for port checks.

## Do next

1. Cap cryptographic signatures (optional Host policy).
2. llvm-cov HTML in CI.

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
