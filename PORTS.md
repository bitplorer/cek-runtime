# Ports — other-language runtimes

Same contract as the Rust kernels. **No third kernel.**

Walkthrough: [GUIDE.md](GUIDE.md). Drivers: [DRIVERS.md](DRIVERS.md).

## Catalog

- **Python Host** (`ports/cek-host-py`)
  - **Role:** decide (mint, verify, project, once, lineage)
  - **Not:** apply, file stores, BoundAsk as a public type
- **JS Peer** (`ports/cek-peer-js`)
  - **Role:** apply + kv / log / DomTree
  - **Not:** mint, verify, refuse
- **TS Peer** (`ports/cek-peer-ts`)
  - **Role:** apply-only (flat ui map)
  - **Not:** mint, DomTree helpers
- **WASM Peer** (`crates/cek-peer-wasm` + `ports/cek-peer-wasm`)
  - **Role:** same Rust Peer kernel, apply-only
  - **Not:** mint

## Completeness vs Rust Host

Python Host **has:**

- action / expiry / sealed-args / scopes / subject
- idempotency before once; once only after project
- HMAC (`cek1:`) and Ed25519 (`ed25519:`, RFC 8032)
- dual-speak `accepted` generations
- lineage reverse + landed-first when `report_receipt`
- Result `digest` (`cek1:` over kind/ops/error)
- 51 CORE vectors (Peer-only fixtures skipped)

Python Host **does not have** (stay on Rust if you need them):

- durable file stores (`FileOnceStore` …)
- `Host::attenuate` / `trust_ed25519` rotation window
- `lower_ops` Baseline lowering

## Completeness vs Rust Peer

JS Peer **has:**

- Baseline apply (`kv.set`, `kv.delete`, `log.append`)
- `ui.dom.morph` / `ui.dom.restore` on a flat map **and** `DomTree`
- `#id`, `/0/1`, insert/remove, text, attr, `html()`
- unknown-Op `skip` / `fail_batch`
- refuse / dispatch_error → zero world change
- stdin runtime (`runtime.mjs`)

JS Peer **must never** mint. CI: `scripts/invariants.sh`.

## Run

```bash
python3 ports/cek-host-py/run_vectors.py crates/cek-contract/vectors
python3 ports/cek-host-py/test_batteries.py
echo '{"action":"kv.write","args":{"key":"a","value":1},"cap":{"id":"c","action":"kv.write","once":false}}' \
  | python3 ports/cek-host-py/runtime.py --now 1000

node ports/cek-peer-js/run-vectors.mjs crates/cek-contract/vectors
node ports/cek-peer-js/batteries.mjs
echo '{"kind":"ok","ops":[{"ns":"kv","name":"set","payload":{"key":"a","value":1}}]}' \
  | node ports/cek-peer-js/runtime.mjs --profile baseline
```

Pipe them: Host stdout Result → Peer stdin. That is the official split.
