# cek-peer-wasm

Apply-only Peer compiled to `wasm32-unknown-unknown`. **No mint.**

WASM ABI hop onto `cek-peer-rust` (the shared JSON apply/receipt door).
Engine stays `Peer::apply` in `cek-peer-kernel`. No second apply
implementation. JSON ABI: `{ result, profile, unknown_op_policy }` →
`{ receipt, kv, ui, log }`.

```bash
# from workspace root
rustup target add wasm32-unknown-unknown
cargo build -p cek-peer-wasm --target wasm32-unknown-unknown --release
node ports/cek-peer-wasm/run-vectors.mjs \
  crates/cek-contract/vectors \
  target/wasm32-unknown-unknown/release/cek_peer_wasm.wasm
```

Same `peer_result` fixtures as `ports/cek-peer-ts`. Host-projected cases stay
on the Rust runner.

Law: https://github.com/bitplorer/cek-framework
