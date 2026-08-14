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

Expected: **103** unit/property tests; **41** vectors PASS; TS Peer apply-only green; demo includes ui.morph restore.

## Current maturity

**Stage C started** (domain `ui.*` pack + snapshot reverse). Stage B still met. See MATURITY.md.

## Done this session (2026-08-14, Stage C)

- Domain pack `ui.morph` → `ui.dom.morph` / `ui.dom.restore`.
- Snapshot lives **on the Op** so landed-first reverse can restore.
- No snapshot → honest `non_reversible`.
- Baseline lowering: `ui.dom.*` → `kv.set` `ui:{target}`.
- Scope attenuation: empty = unrestricted; non-empty allow-list; `Host::attenuate` refuses widen.
- TypeScript apply-only Peer (`ports/cek-peer-ts`) — **no mint** — same `peer_result` vectors.

## Do next

1. Cap cryptographic signatures (optional Host policy).
2. WASM apply-only Peer (same vectors as TS).
3. Richer UI snapshot store (real DOM / prior-value for `kv.delete`).
4. llvm-cov HTML in CI.

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
