# Topology — law vs runtime vs kernel vs driver

Start with **[GUIDE.md](GUIDE.md)** if you want the walkthrough.

Matches [cek-framework](https://github.com/bitplorer/cek-framework) and the runtime [TOPOLOGY](https://github.com/bitplorer/cek-runtime/blob/main/TOPOLOGY.md).

**There is no third kernel. There is no `extensions/` layer.**

```text
cek-framework          LAW          meanings only (other repo)

cek-runtime            RUNTIME      this repo
  crates/cek-contract               wire: Intent, Cap, Op, Result
  crates/cek-host-kernel            HOST KERNEL (decide)
  crates/cek-host-rust              hop: native JSON → host kernel
  crates/cek-peer-kernel            PEER KERNEL (Peer::apply + typed helper)
  crates/cek-ops-baseline           PEER DRIVER  kv
  crates/cek-ops-ui                 PEER DRIVER  ui / DOM world
  crates/cek-peer-wasm              hop: WASM C ABI + own JSON → peer kernel
  crates/cek-peer-rust              hop: native JSON → peer kernel
  crates/cek-peer-pyo3              hop: in-process PyO3 → cek-peer-rust (JSON text A + owned extract B)
  crates/cek-cli                    hop: demo / cek apply → peer-rust / cek host-json → host-rust
  ports/                            other-language apply-only Peers
```

Host decide ownership:

```text
cek-host-kernel          ← Host decide (one engine)
└── cek-host-rust        ← hop: native JSON → host kernel
      └── cek host-json (cli)
```

Peer apply ownership (no sibling→sibling):

```text
cek-peer-kernel          ← Peer::apply (one engine) + apply_world helper
├── cek-peer-wasm        ← hop: WASM C ABI + own JSON → peer kernel
└── cek-peer-rust        ← hop: native JSON → peer kernel
      ├── cek-peer-pyo3
      └── cek apply (cli)
```

PR #10 pointed `cek-peer-wasm` at `cek-peer-rust` so hops shared one JSON
door. That edge was **wrong-owner** and is gone. Each hop serde's the same
field names onto the peer kernel helper. There is no `cek-peer-json` crate.

## Official split

| Name | What it is |
|------|------------|
| **Law** | Cap, Intent, Ops, Host/Peer *roles* — not code |
| **Host kernel** | mint · verify · once · dispatch · lineage · project · reverse |
| **Host runtime** | host kernel + store + keys + clock (this process) |
| **Peer kernel** | profile · apply Ops · receipt · typed apply helper · **no mint** |
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

Driver catalog (payloads, addresses, what a driver must never do): **[DRIVERS.md](DRIVERS.md)**.

## Ports (same roles, other languages)

| Port | Role |
|------|------|
| `ports/cek-host-py` | Historic contract-vector sketch. Published Host is `pip install cek-host` |
| `ports/cek-peer-js` | Peer **runtime** (apply + DomTree). No mint |
| `ports/cek-peer-ts` | Peer apply-only (same contract) |
| `ports/cek-peer-wasm` | WASM hop onto `cek-peer-kernel` (own JSON + C ABI) |
| `ports/cek-peer-pyo3` | PyO3 hop onto `cek-peer-rust` (JSON text A + owned extract B; not a second kernel) |

HMAC / Ed25519 / scopes / dual-speak stay in the **Host kernel** (verify). They are not drivers.
