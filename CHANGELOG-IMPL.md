# Implementation changelog

Law stays in [cek-framework](https://github.com/bitplorer/cek-framework). This file records **runtime** changes only.

## 2026-08-14 — placement consistent with law

### Changed

- `cek-ops-ui` lives under `extensions/` (Peer world), next to `cek-ext-ui` (Host pack).
- Peer kernel apply of `ui.dom.*` is delegated to `cek_ops_ui::apply_op`. Baseline Peer has no UI store.
- [LAYERS.md](LAYERS.md) is the placement map. Dual-speak glossary row restored.

### Unchanged

- Contract may still *name* UI Op shapes for vectors. Kernel still does not project `ui.morph`.
