# JavaScript Peer runtime

Apply-only. Drivers: kv, log, `DomTree`. **No mint.**

Full driver contract: [DRIVERS.md](../../DRIVERS.md).

```bash
node ports/cek-peer-js/run-vectors.mjs crates/cek-contract/vectors
echo '{"kind":"ok","ops":[{"ns":"ui.dom","name":"morph","payload":{"target":"root","patch":{"tag":"h1","attrs":{"id":"root"},"children":[]}}}]}' \
  | node ports/cek-peer-js/runtime.mjs --profile dom
```

`--profile dom` applies `ui.dom.*` onto a tree (`#id`, `/0/1`, insert/remove, `html()`).  
`--profile ui` uses the flat target map (same Ops).  
`--profile baseline` is kv + log only; unknown Ops skip.
