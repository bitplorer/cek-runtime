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

Expected: **123** unit/property tests; **51** vectors PASS; TS + WASM Peers apply-only green.

## Current maturity

Stage C met. Stage D underway. Cap HMAC + subject bind are Host policy.

## Done this session (2026-08-14, subject + llvm-cov)

- `Cap.subject` is enforced: presenter is `args.subject`; mismatch / missing / blank → refuse, zero Ops.
- `scripts/llvm-cov.sh` writes `coverage/summary.txt` + HTML. CI uploads the artifact.

## Do next

1. Real DOM UI store (reference map is enough for now).
2. Ed25519 / multi-key Host policy if a deployment needs it (HMAC is the v0.1 policy).

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
