# Implementation changelog

Law stays in [cek-framework](https://github.com/bitplorer/cek-framework). This file records **runtime** changes only.

## 2026-08-14 — Stage C: ui.* + scopes + TS Peer

### Added

- `cek-contract::ui` — `ui.dom.morph` / `ui.dom.restore`, `lower_to_baseline`, `inverse_ui`.
- `cek-ops-ui::UiStore` — in-memory target → JSON node.
- Host action `ui.morph` / `ui.restore`. Snapshot is **on the morph Op** so landed-first reverse works.
- `Host::lower_ops` — domain → Baseline (`kv.set` `ui:{target}`).
- `Host::attenuate` + `check_scopes` — empty scopes unrestricted; non-empty allow-list; widen refused.
- `Peer::with_ui()` apply-set.
- TypeScript apply-only Peer (`ports/cek-peer-ts`) — no mint.
- Vectors 31 → **41** (`ui_domain`, `attenuation`, Peer-only apply).

### Unchanged (must not regress)

- Cap refuse → zero Ops. BoundAsk private. Peer no mint.
- Once commit after project. Idempotency before once.
- Landed-first reverse. `cek1:` digests. Fail closed.

## 2026-08-14 — coverage, property tables, polish

Property tables, fail-closed store-down, FIPS SHA-256, `FailClosed::default` fix, 31 vectors.

## 2026-08-14 — durable store traits + CORE vectors

Store traits + file backends; 25 CORE vectors; idempotency-before-once.
