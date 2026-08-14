# Implementation changelog

Law stays in [cek-framework](https://github.com/bitplorer/cek-framework). This file records **runtime** changes only.

## 2026-08-14 — WASM apply-only Peer (Stage D)

### Added

- `cek-peer-wasm` wraps `cek-peer-kernel` (`apply_json`). **No mint.**
- `wasm32-unknown-unknown` cdylib (`cek_alloc` / `cek_apply` / `cek_result_ptr`).
- Node runner `ports/cek-peer-wasm/run-vectors.mjs` — same `peer_result` fixtures as TS.
- `Peer::kv_snapshot` / `ui_snapshot` for port checks.

### Unchanged

- Peer (Rust, TS, WASM) cannot mint. Host still decides.

## 2026-08-14 — kv.delete prior-value reverse

Prior on the Op; reverse `kv.set` or honest `non_reversible`.

## 2026-08-14 — Stage C: ui.* + scopes + TS Peer
