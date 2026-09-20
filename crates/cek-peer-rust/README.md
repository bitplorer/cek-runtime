# cek-peer-rust

Shared JSON apply/receipt door for Peer hops. Future Rust peer runtime locus.

Engine stays `Peer::apply` in `cek-peer-kernel`. This crate owns the JSON
port (`apply_json` / `apply_request`). No mint.

Hops (`cek-peer-wasm` ABI, `cek-peer-pyo3`, `cek apply`) call this crate.
They do not own a second apply contract.

JSON: `{ result, profile, unknown_op_policy }` → `{ receipt, kv, ui, log }`.

```toml
cek-peer-rust = "0.1.2"
```
