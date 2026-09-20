# CEK Runtime

Reference **Host** (decide) and **Peer** (apply). No ambient power. Peer never mints.

| Crate | Role |
|-------|------|
| `cek-contract` | Types, S, vectors |
| `cek-host-kernel` | Cap verify → dispatch → lineage → project |
| `cek-host-rust` | hop: native JSON → host kernel |
| `cek-peer-kernel` | Apply S + typed `apply_world` helper |
| `cek-ops-baseline` / `cek-ops-ui` | Drivers |
| `cek-peer-rust` | hop: native JSON → peer kernel |
| `cek-peer-wasm` | hop: WASM C ABI + own JSON → peer kernel |
| `cek-peer-pyo3` | PyO3 hop onto `cek-peer-rust` (apply/receipt only) |
| `cek-cli` | `cek apply` → `cek-peer-rust` · `cek host-json` → `cek-host-rust` |

```bash
cargo test --workspace
cargo run -p cek-cli -- demo
```

Law: [cek-framework](https://github.com/bitplorer/cek-framework) · Python: [cek-python](https://github.com/bitplorer/cek-python)

S = `kv.set` `kv.delete` `log.append` `ui.dom.morph` `ui.dom.restore`. Pair identity is `(ns, name)`.

Publish (crates.io): tag `v*` or Actions → **publish-crates**. Trusted publishers must use environment `crates-io` (see workflow header). `cek-host-rust` is live at **0.1.3** alongside the peer hops.
