# Implementation changelog

Law stays in [cek-framework](https://github.com/bitplorer/cek-framework). This file records **runtime** changes only.

## 2026-08-14 — Cap HMAC (optional Host policy)

### Added

- `Cap.sig` (additive optional). Host `with_hmac_key` mints HMAC-SHA256 (`cek1:`).
- Verify refuses missing/forged sigs **only** when the Host has a key.
- Attenuate re-signs. RFC 4231 known answers.
- Vectors `cap-sig-ok` / `cap-sig-missing` / `cap-sig-tamper` (45 → 48).

### Unchanged

- Unsigned Hosts accept legacy Caps. Peer does not mint or verify. Law unchanged.

## 2026-08-14 — WASM apply-only Peer (Stage D)

`cek-peer-wasm` wraps the Rust Peer. No mint.

## 2026-08-14 — kv.delete prior-value reverse
