# cek-peer-kernel

Apply-only Peer. Applies S (Baseline, or Baseline+UI). No mint.

Unknown ops skip or fail-batch. Runtime stdlib pairs are not applied here — use a Peer runtime driver.

This crate is the **one apply engine** (`Peer::apply`) plus a **typed one-shot
helper** (`apply_world`: profile/policy → `Peer::apply` → receipt + snapshots).
Language hops (`cek-peer-wasm`, `cek-peer-rust`) only serde JSON onto that
helper. They do not share a JSON crate and they do not depend on each other.

```toml
cek-peer-kernel = "0.1.2"
```
