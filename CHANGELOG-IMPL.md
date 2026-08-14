# Implementation changelog

Law stays in [cek-framework](https://github.com/bitplorer/cek-framework). This file records **runtime** changes only.

## 2026-08-14 — kv.delete prior-value reverse

### Added

- `kv_delete_prior` / `inverse_kv`: prior lives **on the Op**.
- Reverse: `kv.delete`+prior → `kv.set`; no prior → `non_reversible`.
- Vectors `kv-delete-prior-reverse`, `kv-delete-no-prior-non-reversible` (43 → 45).

### Unchanged

- Cap refuse → zero Ops. Peer no mint. `log.append` still non-reversible.

## 2026-08-14 — polish: glossary, fail-closed blanks

Action vs Op glossary; empty idempotency / blank scopes refuse.

## 2026-08-14 — Stage C: ui.* + scopes + TS Peer

Domain pack, snapshot reverse, attenuation, TypeScript apply-only Peer.
