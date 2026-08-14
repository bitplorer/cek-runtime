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

Expected: **127** unit/property tests; **54** vectors PASS; TS + WASM Peers apply-only green.

## Current maturity

Stage C met. Stage D: TS + WASM Peers; HMAC **and** Ed25519 Host policy.

## Done this session (2026-08-14, Ed25519)

- `Host::with_ed25519(seed)` mints `ed25519:<hex>` over the same authority bytes as HMAC.
- `trust_ed25519` adds pubs for a rotation window (old Caps still verify).
- Missing / forged / wrong-key → refuse, zero Ops.
- RFC 8032 Test 1 known answer. HMAC Hosts unchanged. Peer never signs.

## Do next

1. Real DOM UI store (reference map is enough for now).
2. Dual-speak law-generation window (institutional).

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
