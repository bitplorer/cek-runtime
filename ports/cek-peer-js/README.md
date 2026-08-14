# JavaScript Peer runtime

Apply-only. Drivers: kv, log, `DomTree`. **No mint.**

```bash
node ports/cek-peer-js/run-vectors.mjs crates/cek-contract/vectors
echo '{"kind":"ok","ops":[{"ns":"ui.dom","name":"morph","payload":{"target":"root","patch":{"tag":"h1","attrs":{"id":"root"},"children":[]}}}]}' \
  | node ports/cek-peer-js/runtime.mjs --profile dom
```
