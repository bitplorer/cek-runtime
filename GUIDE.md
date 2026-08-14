# CEK runtime — guide

One place to understand this tree. Law stays in [cek-framework](https://github.com/bitplorer/cek-framework). This repo is the **runtime**.

## 1. The idea (one minute)

A **Cap** is the only permission.  
The **Host** decides: verify the Cap, then list **Ops** (data).  
The **Peer** applies those Ops. It never mints a Cap.  
If the Host **refuses**, the Peer sees **zero Ops** — the world does not change.

```text
app  →  Host.submit(Intent + Cap)  →  Result { kind, ops[] }
     →  Peer.apply(Result)         →  Receipt { landed, failed }
     →  Host.end_activity(...)     →  reverse Ops (if any)
```

There is **no third kernel**. A bus only moves messages.

## 2. Where code lives

| Official name | Path | Does |
|---------------|------|------|
| Law | cek-framework (other repo) | Meanings |
| Contract | `crates/cek-contract` | Intent, Cap, Op, Result, vectors |
| Host kernel | `crates/cek-host-kernel` | mint, verify, project, once, reverse |
| Peer kernel | `crates/cek-peer-kernel` | apply loop — **no mint** |
| Peer driver | `crates/cek-ops-baseline`, `cek-ops-ui` | kv world, UI/DOM world |

Drivers in detail: **[DRIVERS.md](DRIVERS.md)**. Map: [TOPOLOGY.md](TOPOLOGY.md).

## 3. Words that must not mix

| Say | Means | Do not say |
|-----|--------|------------|
| **Action** | Host verb: `kv.write`, `ui.morph` | An Op |
| **Op** | Peer data: `kv.set`, `ui.dom.morph` | An Intent |
| **Cap** | Permission ticket | A receipt |
| **Driver** | Peer-outer world (kv, DOM) | A kernel |
| **Refuse** | Authority no — **ops = []** | A failed apply |

Full table: [GLOSSARY-IMPL.md](GLOSSARY-IMPL.md).

## 4. Host pipeline (fail closed)

```text
action match → expiry → sealed-args → scopes → subject
  → idempotency lookup (before once)
  → once ensure (do not burn yet)
  → BoundAsk
  → project Ops
  → once commit          ← only if project succeeded
  → lineage (if activity_id)
  → Result
```

Refuse at any verify step → **zero Ops**.  
Unknown action → `dispatch_error`, once-Cap **not** burned.

## 5. What Host projects

| Action | Op | Reverse (when snapshot/prior present) |
|--------|-----|----------------------------------------|
| `kv.write` | `kv.set` | `kv.delete` |
| `kv.delete` | `kv.delete` | `kv.set` of `prior` |
| `log.append` | `log.append` | none (honest non-reversible) |
| `ui.morph` | `ui.dom.morph` | `ui.dom.restore` of `snapshot` |

`ui.morph` is Host **project**. `ui.dom.morph` is the **DOM driver**. Same story, two names.

Driver payloads, addresses (`#id`, `/0/1`), and helpers: [DRIVERS.md](DRIVERS.md).

## 6. Host policy (not law)

These stay on the Host. Peers do not implement them.

- HMAC (`cek1:…`) and Ed25519 (`ed25519:…`) Cap signatures  
- Scopes (narrow only) and subject bind  
- Dual-speak: `accept_generation` for a previous `law_generation`

## 7. Run it

```bash
# Rust kernel
cargo test --workspace
cargo run -p cek-cli -- demo
cargo run -p cek-cli -- vectors crates/cek-contract/vectors
./scripts/invariants.sh
./scripts/batteries.sh          # stress, load, chaos, pen

# Ports (same contract)
python3 ports/cek-host-py/run_vectors.py crates/cek-contract/vectors
node ports/cek-peer-js/run-vectors.mjs crates/cek-contract/vectors
node --experimental-strip-types --no-warnings \
  ports/cek-peer-ts/src/run-vectors.ts crates/cek-contract/vectors
```

Expect about **147** Rust tests, **57** vectors, batteries green.  
Python Host skips Peer-only fixtures (Ed25519 is implemented). JS Peer runs apply-only fixtures.

## 8. Use it from an app

```text
1. Host.mint(id, "kv.write", once, not_after)     # or load a Cap
2. Host.submit({ action, args, cap, activity_id })
3. if kind == ok:  Peer.apply(result)
4. Host.report_receipt(activity_id, receipt.landed)   # optional
5. Host.end_activity(activity_id)  → reverse Ops → Peer.apply again
```

In-process: call the kernels.  
Across processes: send contract JSON (Intent+Cap / Result / receipt).  
Python Host and JS Peer are that split in two languages.

## 9. Never regress

1. Cap refuse → zero mutate Ops  
2. BoundAsk only after verify  
3. Peer has no mint (every language)  
4. Once commit only after successful project  
5. Idempotency lookup before once  
6. Landed-first reverse when a receipt was reported  
7. Digests start with `cek1:`  
8. Fail closed on store down / blank scope / widen / bad sig  

Checks: [INVARIANTS.md](INVARIANTS.md), `scripts/invariants.sh`, `scripts/batteries.sh`.

## 10. Other docs (when you need depth)

| File | Use when |
|------|----------|
| [DRIVERS.md](DRIVERS.md) | kv / log / UI / DOM — Peer-outer only |
| [IMPLEMENTATION.md](IMPLEMENTATION.md) | Pipeline detail, store traits |
| [HARDENING.md](HARDENING.md) | Fail-closed table |
| [EDGE_CASES.md](EDGE_CASES.md) | Expiry, once, reverse edges |
| [TESTING.md](TESTING.md) | How tests are layered |
| [MATURITY.md](MATURITY.md) | Stage A–D checklist |
| [HANDOFF.md](HANDOFF.md) | Next-session prompt |
| [CHANGELOG-IMPL.md](CHANGELOG-IMPL.md) | What changed in the runtime |

Do not add a new layer for new features. Host **project** or Peer **driver**. That is all.
