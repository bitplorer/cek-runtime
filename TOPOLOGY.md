# Topology — law vs runtime vs kernel vs driver

Matches [cek-framework](https://github.com/bitplorer/cek-framework) and the runtime [TOPOLOGY](https://github.com/bitplorer/cek-runtime/blob/main/TOPOLOGY.md).

**There is no third kernel. There is no `extensions/` layer.**

```text
cek-framework          LAW          meanings only (other repo)

cek-runtime            RUNTIME      this repo
  crates/cek-contract               wire: Intent, Cap, Op, Result
  crates/cek-host-kernel            HOST KERNEL (decide)
  crates/cek-peer-kernel            PEER KERNEL (apply loop, no mint)
  crates/cek-ops-baseline           PEER DRIVER  kv
  crates/cek-ops-ui                 PEER DRIVER  ui / DOM world
  crates/cek-cli                    Host+Peer in one process (demo)
  crates/cek-peer-wasm              same Peer kernel, WASM
  ports/                            other-language apply-only Peers
```

## Official split

| Name | What it is |
|------|------------|
| **Law** | Cap, Intent, Ops, Host/Peer *roles* — not code |
| **Host kernel** | mint · verify · once · project · lineage · reverse |
| **Host runtime** | kernel + store + keys + clock (this process) |
| **Peer kernel** | profile · apply Ops · receipt · **no mint** |
| **Peer driver** | the world: kv, UI/DOM, device — **outer**, not a kernel |
| **Contract** | messages between Host and Peer |

```text
L7 app
  → Host runtime → Host kernel     (authority → Result{Ops})
  → Peer runtime → Peer kernel     (apply via drivers)
  → optional receipt → Host kernel
```

`ui.morph` is a **Host action** (kernel project).  
`ui.dom.morph` is an **Op** the **UI/DOM driver** applies.

## Ports (same roles, other languages)

| Port | Role |
|------|------|
| `ports/cek-host-py` | Host **runtime** (verify, project, once) |
| `ports/cek-peer-js` | Peer **runtime** (apply + DomTree). No mint |
| `ports/cek-peer-ts` | Peer apply-only (same contract) |
| `ports/cek-peer-wasm` | Peer apply-only |

HMAC / Ed25519 / scopes / dual-speak stay in the **Host kernel** (verify). They are not drivers.
