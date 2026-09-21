# cek-peer-pyo3

Apply-only Peer compiled as an in-process CPython extension. **No mint.**

PyO3 hop onto `cek-peer-rust`. Engine stays `Peer::apply` via `apply_world`.
No second apply implementation.

Two transports, same algebra (`Op = {ns, name, payload}` → receipt + snapshots):

| Door | Call | Transport |
|------|------|-----------|
| **A** | `PeerAbi.apply(str \| mapping)` | JSON text → `apply_json` (open, locked) |
| **B** | `PeerAbi.apply_ops(ops list or apply-request mapping)` | owned extract → `apply_request` |

Door B does not `json.dumps`. Extract stays in this hop. The peer kernel never
sees a live `PyDict`. GIL is released across `apply_world`.

Lifecycle is host-native across the GIL: **construct → bind → apply →
release**. Import only loads the module. One release door.

```bash
# from workspace root (needs python3-dev / libpython headers)
cargo build -p cek-peer-pyo3 --features extension-module --release
python3 ports/cek-peer-pyo3/run-vectors.py \
  crates/cek-contract/vectors \
  target/release/libcek_peer_pyo3.so
```

Or `bash scripts/run-pyo3-peer.sh` (builds the extension + `cek`, then
compares apply/receipt to `cek apply` on the B-set self-check).

Same `peer_result` fixtures as `ports/cek-peer-wasm`. Host-projected cases
stay on the Rust runner.

This crate is the runtime FFI. It does not tip `bitplorer/cek-python`.

Law: https://github.com/bitplorer/cek-framework
