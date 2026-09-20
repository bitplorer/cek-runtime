# Language ports

| Port | Repo | Notes |
|------|------|--------|
| **Rust** (reference) | this repo | Host/Peer kernels + contract vectors |
| **Python** | [bitplorer/cek-python](https://github.com/bitplorer/cek-python) | `cek-host` (authority) + `cek-surface` (compose, Peer IR, carriers) |
| **Law** | [bitplorer/cek-framework](https://github.com/bitplorer/cek-framework) | not a runtime |

## Python install

```bash
pip install cek-surface   # pulls cek-host
# or monorepo:
# git clone https://github.com/bitplorer/cek-python && pip install -e ./cek-host -e ./cek-surface
```

Peers never mint Caps. See cek-python `docs/ORGANIZATION.md`.

In-process Python apply/receipt ABI (this repo, not a second kernel):
`crates/cek-peer-pyo3` + `ports/cek-peer-pyo3`. Same JSON documents as
`cek-peer-rust` / `cek apply` (wasm ABI is another hop onto that door).
The taught Python carrier remains a follow-up in cek-python.
