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
```

Expected: all unit/property tests pass; **25 vectors PASS**; demo shows refuse → sealed → ok → receipt → reverse.

## Current maturity

Stage B (interop-hardening), moving toward Stage C. See MATURITY.md, HARDENING.md, EDGE_CASES.md, TESTING.md, IMPLEMENTATION.md.

## Done this session (2026-08-14)

- **Durable store traits** — `OnceBackend` / `IdemBackend` / `LineageBackend`
  with in-memory default + JSON file backends (`FileOnceStore`, `FileIdemStore`,
  `FileLineageStore`). Host is backend-agnostic.
- **Idempotency-before-once** — retry of a once-Cap with the same key returns
  the cached Result; same key + different body refuses (no silent fork).
- **CORE vectors 5 → 25** covering cap_verify, single_use, baseline_apply,
  baseline_lowering, unknown_ops, lineage, reverse_on_end, apply_receipt,
  idempotent_submit, trace.
- **Lineage commit-after-ended** now fails closed *before* inserting a ghost cause.

## Do next (priority order)

1. Confirm GitHub `bitplorer/cek-runtime` crates/ match this tree (this session syncs them).
2. Second Peer port (apply-only; no mint) — TypeScript or WASM — against same vectors.
3. Domain Op pack `ui.*` with snapshot reverse class.
4. Cap cryptographic signatures (optional Host policy).
5. Remaining CORE/19 rows: attenuation/scopes, unknown-meta ignore, Peer-no-mint as a vector (CI grep exists).
6. Coverage gate in CI (`cargo llvm-cov`) once toolchain supports it.

## Never regress

- Cap refuse → zero mutate Ops
- BoundAsk only after verify
- Peer has no mint
- Once commit only after successful project
- Idempotency checked **before** once-ensure (retry must not refuse a consumed once-Cap)
- Landed-first reverse when receipt annotated
- Digests `cek1:` stable
- Fail closed on unclear authority

## Prompt starter for next Grok session

```
Continue CEK runtime from this workspace.
Law: https://github.com/bitplorer/cek-framework
Design: https://github.com/bitplorer/cek-runtime
Read HANDOFF.md, IMPLEMENTATION.md, EDGE_CASES.md, HARDENING.md first.
Run cargo test --workspace and vectors before changing code.
Priority: [state your priority from the list above].
Do not violate Cap-only / Ops-as-data / Host-Peer split.
```
