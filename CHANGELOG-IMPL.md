# Implementation changelog

Law stays in [cek-framework](https://github.com/bitplorer/cek-framework). This file records **runtime** changes only.

## 2026-08-14 — durable store traits + CORE vectors

### Added

- `OnceBackend`, `IdemBackend`, `LineageBackend` traits (`cek-host-kernel/src/store.rs`).
- File-backed backends: `FileOnceStore`, `FileIdemStore`, `FileLineageStore`, `FileStores`.
- `Host::with_backends` / `Host::with_stores` for swapping stores.
- `Peer::with_policy` for Skip vs FailBatch.
- Vector runner now executes multi-step cases (`prior_intent`, `peer_apply`, `end_activity`, receipts) with **no CLI special cases**.
- 20 new CORE vectors (5 → 25). Families: cap_verify, single_use, baseline_apply, baseline_lowering, unknown_ops, lineage, reverse_on_end, apply_receipt, idempotent_submit, trace.

### Fixed

- Idempotency same-key + different body now **refuses**. The old fast-path returned the cached Result without comparing digests (silent fork).
- Idempotency is checked **before** once-ensure so a retry of a once-Cap with the same key returns the cached Result instead of `authority_refusal`.
- Lineage `commit` onto an ended Activity is rejected **before** insert (no ghost `by_id` row).
- `sealed-args-mismatch` fixture uses the real `cek1:` bind (no placeholder / runner special case).

### Unchanged (must not regress)

- Cap refuse → zero Ops.
- BoundAsk only after verify + once-ensure.
- Peer has no mint.
- Once commit only after successful project.
- Landed-first reverse when a receipt is annotated.
- Digests stay `cek1:` SHA-256 over canonical JSON.

### Not done

- Second Peer language port.
- Cap signatures.
- `ui.*` snapshot reverse.
- Scope attenuation.
- Multi-process file lock / SQL backends.
