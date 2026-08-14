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
```

Expected: **87** unit/property tests pass; **31** vectors PASS; demo shows refuse → sealed → ok → receipt → reverse; invariants script exits 0.

## Current maturity

Stage B (interop-hardening), moving toward Stage C. See MATURITY.md, HARDENING.md, EDGE_CASES.md, TESTING.md, IMPLEMENTATION.md, INVARIANTS.md.

## Done this session (2026-08-14, polish)

- Property tables expanded (idempotency, sealed, reverse, lowering, trace, once+retry).
- Fail-closed tests: store-down backends, concurrent once (exactly one `ok`).
- FIPS SHA-256 known-answer tests (digest stability).
- `FailClosed::default()` now matches serde (`once_store_down: true`).
- Vectors 25 → **31** (`unknown_meta`, expiry boundary, non-reversible log, empty activity_id, …).
- `scripts/invariants.sh` + coverage inventory; CI runs both.
- [INVARIANTS.md](INVARIANTS.md) maps each never-regress rule to a test.

## Do next (priority order)

1. Second Peer port (apply-only; no mint) — TypeScript or WASM — against same vectors.
2. Domain Op pack `ui.*` with snapshot reverse class.
3. Cap cryptographic signatures (optional Host policy).
4. Remaining CORE/19 row: attenuation/scopes.
5. llvm-cov HTML in CI once `cargo-llvm-cov` is pinned.

## Never regress

- Cap refuse → zero mutate Ops
- BoundAsk only after verify
- Peer has no mint
- Once commit only after successful project
- Idempotency checked **before** once-ensure
- Landed-first reverse when receipt annotated
- Digests `cek1:` stable (FIPS SHA-256)
- Fail closed on unclear authority / store down

## Prompt starter for next Grok session

```
Continue CEK runtime from this workspace.
Law: https://github.com/bitplorer/cek-framework
Design: https://github.com/bitplorer/cek-runtime
Read HANDOFF.md, IMPLEMENTATION.md, EDGE_CASES.md, HARDENING.md, INVARIANTS.md first.
Run cargo test --workspace, vectors, and ./scripts/invariants.sh before changing code.
Priority: [state your priority from the list above].
Do not violate Cap-only / Ops-as-data / Host-Peer split.
```
