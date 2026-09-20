# cek-peer-wasm

Apply-only Peer compiled to `wasm32-unknown-unknown`. **No mint.**

WASM hop onto `cek-peer-kernel` (C ABI + this crate's thin JSON wire).
Engine stays `Peer::apply` via `apply_world`. No second apply
implementation. Does not depend on `cek-peer-rust`. JSON ABI:
`{ result, profile, unknown_op_policy }` → `{ receipt, kv, ui, log }`.

**C ABI honesty:** `cek_apply` rustdoc says length or `-1` on error. `-1` is only
null pointer / non-UTF-8. `apply_json` `Err` is returned as a **positive-length
error string** (Err-as-body). HOLD: do not reshape the C ABI. Details:
[crate README](../../crates/cek-peer-wasm/README.md).

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
