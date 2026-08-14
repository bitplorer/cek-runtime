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

Expected: **130** unit/property tests; **57** vectors PASS.

## Current maturity

Kernel is Baseline only. `ui.*` is the `cek-ext-ui` extension (`Host::with_pack`).

## Done this session (2026-08-14, extensions)

- `DomainPack` hook on Host. Kernel `project_baseline` is kv/log only.
- `extensions/cek-ext-ui`: `UiPack` + optional `DomTree`. No mint.
- CLI / vectors register the pack. Kernel without pack: `ui.morph` is dispatch_error.

See [LAYERS.md](LAYERS.md) for law / kernel / extension placement.

## Done this session (2026-08-14, placement)

- `cek-ops-ui` moved to `extensions/` (Peer world).
- Peer Baseline has no UI store; `with_ui()` is the extension profile.
- Dual-speak glossary row restored.

## Do next

New domain features go in `extensions/`. Do not add them to Host/Peer kernels.

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
