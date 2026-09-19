# cek-peer-pyo3

Same `cek-peer-kernel`, JSON/PyO3 ABI. Apply/receipt only. No mint.

Uses the wasm JSON documents: `{ result, profile, unknown_op_policy }` →
`{ receipt, kv, ui, log }`. Not a second Peer kernel and not a Cap mint
surface.

```toml
cek-peer-pyo3 = "0.1.2"
```
