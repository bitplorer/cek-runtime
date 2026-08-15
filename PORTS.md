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
