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
python3 ports/cek-host-py/run_vectors.py crates/cek-contract/vectors
node ports/cek-peer-js/run-vectors.mjs crates/cek-contract/vectors
```

Expected: **261** `#[test]`; **72** vector JSON files (how counted: [TESTING.md](TESTING.md)); batteries (`./scripts/batteries.sh`) green.

## Peer hops (#11) / Host runtime door

One apply engine: `cek-peer-kernel` `Peer::apply` + `apply_world`.  
wasm and rust are siblings on the peer kernel (`hop: native JSON → peer kernel` / `hop: WASM C ABI + own JSON → peer kernel`). PyO3 and `cek apply` hop onto rust.  
No second Peer engine. No wasm→rust dep (that was #10, wrong-owner, gone).

Host: `cek-host-rust` is `hop: native JSON → host kernel`.  
`cek host-json` hops onto that crate. Not a third kernel. Not a Python Host twin.  
Map: [TOPOLOGY.md](TOPOLOGY.md).

## Current maturity

Official topology + ports: Python Host runtime, JS Peer runtime, TS/WASM/PyO3 apply hops, DOM tree driver.

## Done this session

Aligned placement with cek-framework / cek-runtime meta. Removed the invented `extensions/` layer.

## Do next

Follow official topology. Domain *worlds* are Peer drivers (`cek-ops-*`). Host *project* stays in the Host kernel.

## Never regress

- Cap refuse → zero mutate Ops
- BoundAsk only after verify
- Peer has no mint (Rust **and** TS)
- Once commit only after successful dispatch
- Idempotency before once-ensure
- Landed-first reverse when receipt annotated
- Digests `cek1:` (FIPS SHA-256)
- Fail closed on unclear authority / store down / scope deny / attenuate widen
- Snapshot reverse only when snapshot is present (else mark non-reversible)

## Prompt starter

```
Read **GUIDE.md**, then HANDOFF.md, IMPLEMENTATION.md, INVARIANTS.md.
Run cargo test, vectors, invariants, batteries before changing code.
Do not add a third kernel. Host project or Peer driver only.
```
