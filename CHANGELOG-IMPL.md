# Implementation changelog

Law stays in [cek-framework](https://github.com/bitplorer/cek-framework). This file records **runtime** changes only.

## 2026-08-14 — Ed25519 Host policy

### Added

- `Host::with_ed25519` / `trust_ed25519`. Mint attaches `ed25519:<hex>` over `cap_sign_bytes`.
- Rotation: a Host can trust more than one public key.
- RFC 8032 Test 1 known answer. Vectors `ed25519-ok` / `missing` / `tamper` (51 → 54).

### Unchanged

- HMAC Hosts unchanged. Unsigned Hosts accept legacy Caps. Peer does not mint or sign.

## 2026-08-14 — subject bind + llvm-cov HTML

`Cap.subject` / `args.subject`. llvm-cov HTML in CI.
