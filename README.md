# CEK Runtime

Reference **Host** (decide) and **Peer** (apply). No ambient power. Peer never mints.

| Crate | Role |
|-------|------|
| `cek-contract` | Types, S, vectors |
| `cek-host-kernel` | Cap verify → dispatch → lineage → project |
| `cek-peer-kernel` | Apply S only |
| `cek-ops-baseline` / `cek-ops-ui` | Drivers |
| `cek-peer-rust` | Shared JSON apply/receipt door (Rust peer runtime locus) |
| `cek-peer-wasm` | WASM ABI hop onto `cek-peer-rust` |
| `cek-peer-pyo3` | PyO3 hop onto `cek-peer-rust` (apply/receipt only) |
| `cek-cli` | `cek demo` · `cek vectors` · `cek apply` · `cek host-json` |

```bash
cargo test --workspace
cargo run -p cek-cli -- demo
```

Law: [cek-framework](https://github.com/bitplorer/cek-framework) · Python: [cek-python](https://github.com/bitplorer/cek-python)

S = `kv.set` `kv.delete` `log.append` `ui.dom.morph` `ui.dom.restore`. Pair identity is `(ns, name)`.

Publish (crates.io): tag `v*` or Actions → **publish-crates**. Trusted publishers must use environment `crates-io` (see workflow header).
