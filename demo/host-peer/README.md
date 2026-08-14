# Demo app — Python Host → JS Peer

One **L7 app**. It is not a kernel.

```text
orchestrate.py  (this app)
    │  Intent + Cap          contract JSON
    ▼
Python Host                 decide → Result { kind, ops[] }
    │  Result                contract JSON (one line per step)
    ▼
JS Peer session             apply → receipt + world
    │  receipt               contract JSON
    ▼
Python Host.report_receipt / end_activity
    │  reverse Result
    ▼
JS Peer                     apply inverse Ops
```

## Run

```bash
bash demo/host-peer/run.sh
```

Writes `demo/host-peer/out/trace.json` and `demo/host-peer/out/index.html`.

## Scenes

1. **Refuse** — Cap action ≠ Intent action. `ops=[]`. Peer world empty.
2. **kv.write** — Host projects `kv.set`. Peer `greeting=hello`.
3. **ui.morph** — Host projects `ui.dom.morph`. Peer `<h1>Hello</h1>`.
4. **Receipt** — Peer landed Ops annotated on the Host.
5. **Reverse** — `end_activity` lists inverse Ops. Peer undoes kv + DOM.
6. **Once-Cap** — second submit refuses; world stays at first write.
7. **Expired Cap** — `not_after` in the past; Peer never sees `late`.

## Contract JSON (the only wire)

- **Intent + Cap** — app → Host (`action`, `args`, `cap`, optional `activity_id`)
- **Result** — Host → Peer (`kind`, `ops`, `error`, `digest`)
- **Receipt** — Peer → Host (`landed`, `failed`)

The demo keeps one JS Peer process and speaks **NDJSON** of those objects. A queue or HTTP would carry the same bytes.

## Files

- `orchestrate.py` — L7 story + Host
- `peer_session.mjs` — long-lived Peer (no mint)
- `run.sh` — one command
- `out/` — generated report (gitignored)

See [PORTS.md](../../PORTS.md) and [GUIDE.md](../../GUIDE.md).
