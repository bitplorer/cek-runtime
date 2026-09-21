# cek-peer-pyo3

PyO3 hop onto `cek-peer-rust`. Apply/receipt only. No mint.

Same Cap algebra (`Op = {ns, name, payload}` → `Peer::apply` via `apply_world`).
Two transports on this hop:

| Door | Python | Rust | Transport |
|------|--------|------|-----------|
| **A** | `PeerAbi.apply(str \| mapping)` | `apply_json` | JSON text (open, parity-locked) |
| **B** | `PeerAbi.apply_ops(list[dict] \| apply-request mapping)` | `apply_request` | owned extract in this hop |

Door B extracts `{ns, name, payload}` (and request envelope fields) into owned
Rust types, releases the GIL, then calls typed apply. No live `PyDict` in
`cek-peer-kernel`. Door A stays open.

```toml
cek-peer-pyo3 = "0.1.3"
```
