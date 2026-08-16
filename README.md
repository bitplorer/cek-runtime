# CEK Runtime

Reference **Host** (decide) and **Peer** (apply). No ambient power. Peer never mints.

| Crate | Role |
|-------|------|
| `cek-contract` | Types, S, vectors |
| `cek-host-kernel` | Cap verify → project → lineage |
| `cek-peer-kernel` | Apply S only |
| `cek-ops-baseline` / `cek-ops-ui` | Drivers |
| `cek-peer-wasm` | Same Peer kernel, WASM ABI |
| `cek-cli` | `cek demo` · `cek vectors` · `cek apply` · `cek host-json` |

```bash
cargo test --workspace
cargo run -p cek-cli -- demo
```

Law: [cek-framework](https://github.com/bitplorer/cek-framework) · Python: [cek-python](https://github.com/bitplorer/cek-python)

S = `kv.set` `kv.delete` `log.append` `ui.dom.morph` `ui.dom.restore`. Pair identity is `(ns, name)`.

Publish (crates.io): tag `v*` or Actions → **publish-crates**. Trusted publishers must use environment `crates-io` (see workflow header).
