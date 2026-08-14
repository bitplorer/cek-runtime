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

Expected: **120** unit/property tests; **48** vectors PASS; TS + WASM Peers apply-only green.

## Current maturity

**Stage C met.** **Stage D underway** — TS + WASM Peers; Cap HMAC is optional Host policy (not law).

## Done this session (2026-08-14, Cap HMAC)

- Optional Host policy: `Host::with_hmac_key`. Mint attaches `cek1:` HMAC; verify refuses missing/forged sigs.
- Unsigned Caps still work when the Host has no key (existing vectors unchanged).
- Attenuate re-signs the child. Peer never verifies or mints.
- RFC 4231 HMAC-SHA256 known answers.

## Do next

1. llvm-cov HTML in CI.
2. Real DOM UI store (reference map is enough for now).
3. Ed25519 / multi-key Host policy if a deployment needs it (HMAC is the v0.1 policy).

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
