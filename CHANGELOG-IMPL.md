# Implementation changelog

Law stays in [cek-framework](https://github.com/bitplorer/cek-framework). This file records **runtime** changes only.

## 2026-08-14 — topology matches official meta

Dropped the invented `extensions/` layer (it was a third home). Official map:

- **Host kernel** projects Actions (`ui.morph` included)
- **Peer kernel** applies Ops; **no mint**
- **`cek-ops-ui`** is a Peer **driver** (outer), not a pack kernel
- [TOPOLOGY.md](TOPOLOGY.md) matches cek-runtime / cek-framework
