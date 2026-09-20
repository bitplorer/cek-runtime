# cek-peer-wasm

`hop: WASM C ABI + own JSON → peer kernel`

Same engine (`Peer::apply` via `apply_world` on `cek-peer-kernel`). No mint.
Not a second apply contract. Does **not** depend on `cek-peer-rust`
(#10 wrong-owner, restored).

### C ABI honesty (`cek_apply`)

The rustdoc on `cek_apply` says: return result length (`>= 0`) or **`-1` on error**.

What the function actually does (HOLD: do not reshape this C ABI):

| Input | Return | Body (`cek_result_ptr`) |
|-------|--------|-------------------------|
| null pointer | `-1` | none |
| non-UTF-8 bytes | `-1` | none |
| `apply_json` `Err` (bad JSON / serde) | **positive length** | the **error string** (Err-as-body) |
| `apply_json` `Ok` | positive length | JSON `{ receipt, kv, ui, log }` |

JSON/apply failures are leftover Err-as-body, not the documented `-1`. The Node runner (`ports/cek-peer-wasm/run-vectors.mjs`) only treats `n < 0` as failure, so an Err-as-body payload is parsed as JSON and throws at decode — it is not a second error protocol.

```toml
cek-peer-wasm = "0.1.3"
```
