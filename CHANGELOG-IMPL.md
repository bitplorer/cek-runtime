# Implementation changelog

Law stays in [cek-framework](https://github.com/bitplorer/cek-framework). This file records **runtime** changes only.

## 2026-08-14 — polish: glossary, fail-closed blanks

### Added

- `cek_contract::actions` — Intent verbs vs Ops (`ACTION_UI_MORPH` ≠ `ui.dom.morph`).
- [GLOSSARY-IMPL.md](GLOSSARY-IMPL.md).
- Refuse empty `idempotency_key` and blank scope tokens.
- `FailClosed` handshake: `idem_store_down`, `sealed_args`, `scopes` (default true).
- Vectors: `empty-idempotency-key`, `empty-scope-token` (41 → 43).

### Unchanged

- Cap refuse → zero Ops. Peer no mint. Once after project. `cek1:` digests.

## 2026-08-14 — Stage C: ui.* + scopes + TS Peer

Domain pack, snapshot reverse, attenuation, TypeScript apply-only Peer.

## 2026-08-14 — coverage, property tables, polish

Property tables, fail-closed store-down, FIPS SHA-256, `FailClosed::default` fix.

## 2026-08-14 — durable store traits + CORE vectors

Store traits + file backends; idempotency-before-once.
