# cek-peer-wasm

WASM hop: C ABI plus this crate's thin JSON apply/receipt wire on
`cek-peer-kernel`. Same engine (`Peer::apply` via `apply_world`). No mint.
Not a second apply contract. Does **not** depend on `cek-peer-rust`
(#10 wrong-owner, restored).

```toml
cek-peer-wasm = "0.1.2"
```
