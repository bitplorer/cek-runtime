# CEK Runtime

**Implement a CEK Host (authority) and Peer (apply surface) without ambient power.**

| | |
|--|--|
| **This repo** | Design playbook **+** reference Rust workspace (`crates/`) |
| **Law** | [cek-framework](https://github.com/bitplorer/cek-framework) |

| Status | Detail |
|--------|--------|
| **v0.1 code** | Contract types, Host (BoundAsk), Peer (apply-only), Baseline kv, vectors, CLI |
| Proven | Cap refuse → zero Ops; once-Cap; `kv.set` land; Activity reverse |
| Doc | [IMPLEMENTATION.md](IMPLEMENTATION.md) |

```bash
cargo test --workspace
cargo run -p cek-cli -- demo
cargo run -p cek-cli -- vectors crates/cek-contract/vectors
```

---

## This repo at a glance

**What this is:** how to **build** Host and Peer. Not new law.

| Piece | Role |
|-------|------|
| **cek-contract** | Types + vectors — sole interop product |
| **Host kernel** | mint, verify, once, BoundAsk, lineage, project, reverse |
| **Peer kernel** | apply Ops only — **no** mint |
| **Wire / in-proc** | Contract messages — no third kernel |

**Submit (fail closed)**

```text
verify → once → project → lineage → Result
```

**Reverse** on Activity end / Cap revoke — **not** when apply finishes.

| This repo **is** | This repo **is not** |
|------------------|----------------------|
| Host/Peer structure, contract, reference Rust | A new set of axioms |
| Topology and pipelines | A central “CEK cloud” service |
| Rust reference kernels | Python/TS Host kernels |

[TOPOLOGY.md](TOPOLOGY.md) · [CONCEPTS.md](CONCEPTS.md) · [IMPLEMENTATION.md](IMPLEMENTATION.md)

---

## Problems this solves

| You’re building | Risk without CEK runtime | This design |
|-----------------|--------------------------|-------------|
| **Agent tools** | Invented permission; weak revoke | Cap verify; Ops under lineage |
| **UI / DOM channel** | Free client mutate; guesswork undo | Peer applies `dom.*`; Host reverse on end |
| **Device / robot** | Trusted bypass | Refuse → no effect Op |
| **Multi-version clients** | Flag-day breaks | Baseline + profile projection |
| **Cancel / unload / revoke** | Fake rollback | Reverse plan or honest mark |
| **Authority PR review** | Cap-skip flags | Vectors + Peer no-mint |

**DOM:** morph **stays** until Activity end or Cap revoke — apply complete ≠ undo.

---

## Design rule

**Contract is the sole interop product → Host is a Cap state machine → Peer is pure apply → reverse before rich Ops → vectors gate merge.**

| Need | Open |
|------|------|
| Axioms and vocabulary | [cek-framework](https://github.com/bitplorer/cek-framework) |
| Structure, topology, CI, crates | **This repo** |
| Product handlers | Your app — hold Caps, call `submit` |

---

## Navigate

| Path | Role |
|------|------|
| [IMPLEMENTATION.md](IMPLEMENTATION.md) | Runnable slice docs |
| [CONCEPTS.md](CONCEPTS.md) | Implementation concepts |
| [TOPOLOGY.md](TOPOLOGY.md) | Runtime contains kernel; wire |
| [SCOPE.md](SCOPE.md) | Law vs this repo |
| `crates/` | Rust workspace |
| [00-contract/](00-contract/) | Contract design notes |
| [INDEX.md](INDEX.md) | Full map |

## Definition of done (v0.1 met for core path)

1. Contract types + vector JSON fixtures  
2. Rust Host + Peer unit tests green  
3. Baseline Ops end-to-end in `cek demo`  
4. Cap refuse → zero mutate effects  
5. Activity end → reverse Ops  
6. Peer exposes **no** mint API  
