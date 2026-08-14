# Implementation changelog

Law stays in [cek-framework](https://github.com/bitplorer/cek-framework). This file records **runtime** changes only.

## 2026-08-14 — coverage, property tables, polish

### Added

- Host property tables: kv.delete / log.append lowering, reverse inverse, idempotency replay+conflict, sealed bind, trace-not-authority, once-not-burned, once+idem retry.
- Fail-closed suite: down once/idem/lineage backends; concurrent once (exactly one `ok`); empty `end_activity`.
- FIPS SHA-256 known answers (`""`, `"abc"`, 56-byte).
- Contract property tests: serde roundtrip, unknown JSON fields ignored, Baseline catalog, vector checker errors.
- Peer: delete/log, dispatch_error no-mutate, malformed payload, refuse-never-mutates world.
- `cek-ops-baseline` KvStore tests.
- Vectors 25 → 31: `unknown_meta`, expiry-at-boundary, non-reversible log.append, empty activity_id, missing log message, peer dispatch_error.
- `scripts/invariants.sh`, coverage inventory in `scripts/coverage.sh`, [INVARIANTS.md](INVARIANTS.md).

### Fixed

- `FailClosed::default()` now sets `once_store_down: true` (serde already did; `Default` derive had `false`).

### Unchanged (must not regress)

- Cap refuse → zero Ops.
- BoundAsk only after verify + once-ensure.
- Peer has no mint.
- Once commit only after successful project.
- Landed-first reverse when a receipt is annotated.
- Digests stay `cek1:` SHA-256 over canonical JSON.

## 2026-08-14 — durable store traits + CORE vectors

### Added

- `OnceBackend`, `IdemBackend`, `LineageBackend` traits + file-backed JSON backends.
- Idempotency-before-once; generic vector runner; 25 CORE vectors.

### Fixed

- Idempotency same-key + different body now refuses (was silent fork).
- Lineage commit onto an ended Activity rejected before insert.

### Not done

- Second Peer language port.
- Cap signatures.
- `ui.*` snapshot reverse.
- Scope attenuation.
- Multi-process file lock / SQL backends.
