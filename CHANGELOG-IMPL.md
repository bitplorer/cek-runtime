# Implementation changelog

Law stays in [cek-framework](https://github.com/bitplorer/cek-framework). This file records **runtime** changes only.

## 2026-08-14 — subject bind + llvm-cov HTML

### Added

- `Cap.subject` enforced via `args.subject` (Host policy). Mismatch / missing / blank → refuse, zero Ops.
- `scripts/llvm-cov.sh` → `coverage/summary.txt` + HTML. CI uploads `coverage/`.
- Vectors `subject-bind-ok` / `subject-bind-mismatch` / `subject-bind-missing` (48 → 51).

### Unchanged

- Unset `Cap.subject` remains unrestricted. Peer still has no mint.

## 2026-08-14 — Cap HMAC (optional Host policy)

`Cap.sig` + `Host::with_hmac_key`. Unsigned Hosts accept legacy Caps.
