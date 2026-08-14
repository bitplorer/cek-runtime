# cek-peer-wasm

Apply-only Peer compiled to `wasm32-unknown-unknown`. **No mint.**

Uses the same Rust `cek-peer-kernel` as the native Peer (no second apply
implementation). JSON ABI: `{ result, profile, unknown_op_policy }` →
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
