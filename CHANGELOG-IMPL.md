# Implementation changelog

Law stays in [cek-framework](https://github.com/bitplorer/cek-framework). This file records **runtime** changes only.

## 2026-08-14 — dual-speak law-generation window

### Added

- `Cap.law_generation` (additive). Unset = legacy current.
- `Host::accept_generation` / Manifest `accepted_generations`.
- Unknown or blank generation → refuse, zero Ops.
- Vectors `law-gen-unknown` / `law-gen-accepted` / `law-gen-blank` (54 → 57).

### Unchanged

- Law is still `cek-law-1`. This is a Host window, not a new generation.

## 2026-08-14 — Ed25519 Host policy

`with_ed25519` / `trust_ed25519`. RFC 8032 Test 1.
