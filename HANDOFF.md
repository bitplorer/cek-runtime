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

python3 ports/cek-host-py/run_vectors.py crates/cek-contract/vectors
node ports/cek-peer-js/run-vectors.mjs crates/cek-contract/vectors

Expected: **131** Rust tests; **57** vectors; Python Host 48 pass (9 skip); JS Peer apply-only green.

## Current maturity

Official topology + ports: Python Host runtime, JS Peer runtime, DOM tree driver.

## Done this session

Aligned placement with cek-framework / cek-runtime meta. Removed the invented `extensions/` layer.

## Do next

Follow official topology. Domain *worlds* are Peer drivers (`cek-ops-*`). Host *project* stays in the Host kernel.

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
