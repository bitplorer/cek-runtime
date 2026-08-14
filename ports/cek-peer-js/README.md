# JavaScript Peer runtime

Apply-only. Drivers: kv, log, `DomTree`. **No mint. No Cap verify.**

Completeness: [PORTS.md](../../PORTS.md). Drivers: [DRIVERS.md](../../DRIVERS.md).

## Has

- Baseline Ops: `kv.set`, `kv.delete`, `log.append`
- `ui.dom.morph` / `ui.dom.restore` (flat map + tree)
- `DomTree`: `#id`, `/0/1`, insert, remove, text, attr, `html()`
- Unknown-Op policy: `skip` (default) or `fail_batch`
- Refusal / dispatch_error → empty receipt, world unchanged

## Must not

- `mint`, verify Caps, attach signatures

## Run

```bash
node ports/cek-peer-js/run-vectors.mjs crates/cek-contract/vectors
node ports/cek-peer-js/batteries.mjs
echo '{"kind":"ok","ops":[{"ns":"ui.dom","name":"morph","payload":{"target":"root","patch":{"tag":"h1","attrs":{"id":"root"},"children":[]}}}]}' \
  | node ports/cek-peer-js/runtime.mjs --profile dom
```

- `--profile baseline` — kv + log  
- `--profile ui` — plus flat UI map  
- `--profile dom` — plus `DomTree`
