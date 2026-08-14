# Implementation changelog

Law stays in [cek-framework](https://github.com/bitplorer/cek-framework). This file records **runtime** changes only.

## 2026-08-14 — extensions (ui out of kernel)

### Added

- `DomainPack` + `Host::with_pack`. Kernel Baseline is kv/log only.
- `extensions/cek-ext-ui`: `UiPack` (project/inverse) + optional `DomTree`.
- Without the pack, `ui.morph` is `dispatch_error` (zero Ops). CLI registers the pack.

### Unchanged

- Vectors still cover `ui.*` via the registered pack. Peer still has no mint. Law unchanged.

## 2026-08-14 — dual-speak law-generation window
