# Topology — law vs runtime vs kernel vs driver

Start with **[GUIDE.md](GUIDE.md)** if you want the walkthrough.

Matches [cek-framework](https://github.com/bitplorer/cek-framework) and the runtime [TOPOLOGY](https://github.com/bitplorer/cek-runtime/blob/main/TOPOLOGY.md).

**There is no third kernel. There is no `extensions/` layer.**

```text
cek-framework          LAW          meanings only (other repo)

cek-runtime            RUNTIME      this repo
  crates/cek-contract               wire: Intent, Cap, Op, Result
  crates/cek-host-kernel            HOST KERNEL (decide)
  crates/cek-peer-kernel            PEER KERNEL (apply loop, no mint)
  crates/cek-peer-rust              JSON apply door (Rust peer runtime locus)
  crates/cek-ops-baseline           PEER DRIVER  kv
  crates/cek-ops-ui                 PEER DRIVER  ui / DOM world
  crates/cek-cli                    hop: Host+Peer demo / cek apply
  crates/cek-peer-wasm              hop: WASM ABI → cek-peer-rust
  crates/cek-peer-pyo3              hop: in-process PyO3 → cek-peer-rust
  ports/                            other-language apply-only Peers
```

## Official split

| Name | What it is |
|------|------------|
| **Law** | Cap, Intent, Ops, Host/Peer *roles* — not code |
| **Host kernel** | mint · verify · once · dispatch · lineage · project · reverse |
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

Driver catalog (payloads, addresses, what a driver must never do): **[DRIVERS.md](DRIVERS.md)**.

## Ports (same roles, other languages)

| Port | Role |
|------|------|
| `ports/cek-host-py` | Historic contract-vector sketch. Published Host is `pip install cek-host` |
| `ports/cek-peer-js` | Peer **runtime** (apply + DomTree). No mint |
| `ports/cek-peer-ts` | Peer apply-only (same contract) |
| `ports/cek-peer-wasm` | WASM hop onto `cek-peer-rust` |
| `ports/cek-peer-pyo3` | PyO3 hop onto `cek-peer-rust` (not a second kernel) |

HMAC / Ed25519 / scopes / dual-speak stay in the **Host kernel** (verify). They are not drivers.
