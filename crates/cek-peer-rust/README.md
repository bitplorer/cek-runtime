# cek-peer-rust

Native Rust peer runtime. Thin JSON apply/receipt wire on `cek-peer-kernel`.

Engine stays `Peer::apply` (via `apply_world`). This crate owns **its own**
JSON port (`apply_json` / `apply_request`). No mint. Not a second kernel.

`cek-peer-pyo3` and `cek apply` hop onto this crate. `cek-peer-wasm` does
**not** — it owns a sibling JSON wire on the same kernel helper. PR #10's
wasm→rust edge was wrong-owner and is gone.

JSON: `{ result, profile, unknown_op_policy }` → `{ receipt, kv, ui, log }`.

```toml
cek-peer-rust = "0.1.3"
```
