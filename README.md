# CEK Runtime

**Implement a CEK Host (authority) and Peer (apply surface) without ambient power.**

| | |
|--|--|
| **This repo** | Implementation playbook — contract, pipelines, topology, CI, Rust shape |
| **Law** | [cek-framework](https://github.com/bitplorer/cek-framework) |

| Status | Detail |
|--------|--------|
| Done | Design, diagrams, concept/topology docs |
| Next | Runnable crates + JSON vectors |

---

## This repo at a glance

**What this is:** how to **build** Host and Peer. Not new law.

| Piece | Role |
|-------|------|
| **cek-contract** | Schemas + vectors — sole interop product |
| **Host runtime** | Process that **contains** the Host kernel |
| **Host kernel** | mint, verify, once, dispatch, lineage, project, reverse |
| **Peer runtime** | Process that **contains** the Peer kernel |
| **Peer kernel** | apply Ops only — **no** mint |
| **Wire / in-proc** | Contract messages only — no third kernel in the middle |

**Submit (fail closed)**

```text
verify → once → truth → dispatch → lineage → project → Result
```

**Reverse** runs on Activity end or Cap revoke — **not** when apply finishes.  
**CI:** red vectors or Peer mint symbol → block merge.

| Need | Open |
|------|------|
| Law meanings | [cek-framework](https://github.com/bitplorer/cek-framework) |
| This playbook | **Here** → Host/Peer crates |

| This repo **is** | This repo **is not** |
|------------------|----------------------|
| Host/Peer structure, contract, CI | A new set of axioms |
| Topology and pipelines | A central “CEK cloud” service |
| Rust reference kernels | Python/TS Host kernels |

| Piece | One line |
|-------|----------|
| **BoundAsk** | After verify; required for dispatch |
| **Receipt** | What Peer landed; guides reverse |
| **Baseline Ops** | Forever-interop data effects |

<details>
<summary>ASCII overview (wide screens)</summary>

```text
┌────────────────────────── cek-runtime ──────────────────────────┐
│  IMPLEMENTATION (how to build; not new law)                     │
│                                                                 │
│  cek-contract     schemas + vectors = sole interop product      │
│  Host runtime   ⊃ Host kernel   (mint, verify, lineage, …)      │
│  Peer runtime   ⊃ Peer kernel   (apply Ops only; no mint)       │
│  Wire / in-proc   contract messages — no third kernel in middle │
│                                                                 │
│  submit: verify → once → truth → dispatch → lineage → project   │
│  reverse: on Activity end / Cap revoke (not when apply finishes)│
│  CI: red vectors or Peer mint symbol → block merge              │
└─────────────────────────────────────────────────────────────────┘
         │ law                                    │ this playbook
         ▼                                        ▼
   cek-framework                            Host/Peer crates
```

```text
┌──────────────────────── Host runtime ────────────────────────┐
│  transport · once-store · lineage DB · Cap keys · clock        │
│  ┌────────────────── Host kernel ──────────────────────────┐ │
│  │ mint · verify · once · dispatch · lineage · project ·    │ │
│  │ reverse · Result                                         │ │
│  └──────────────────────────────────────────────────────────┘ │
└───────────────────────────────┬──────────────────────────────┘
                                │ wire: Intent+Cap / Result / receipt
┌───────────────────────────────▼──────────────────────────────┐
│  transport · apply drivers (DOM, kv, device, …)               │
│  ┌────────────────── Peer kernel ──────────────────────────┐ │
│  │ profile · apply Ops · optional receipt · NO mint         │ │
│  └──────────────────────────────────────────────────────────┘ │
└──────────────────────── Peer runtime ────────────────────────┘
```

</details>

[TOPOLOGY.md](TOPOLOGY.md) · [CONCEPTS.md](CONCEPTS.md) · [law CONCEPTS](https://github.com/bitplorer/cek-framework/blob/main/CONCEPTS.md)

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

Host and Peer kernels here: **Rust only**. Other languages = callers or later Peer ports.

---

## Navigate

| Path | Role |
|------|------|
| [CONCEPTS.md](CONCEPTS.md) | Implementation concepts |
| [TOPOLOGY.md](TOPOLOGY.md) | Runtime contains kernel; wire |
| [SCOPE.md](SCOPE.md) | Law vs this repo |
| [00-contract/](00-contract/) | Schemas, vectors, Baseline |
| [01-kernels/](01-kernels/) | Host/Peer APIs |
| [02-host-pipeline/](02-host-pipeline/) | Submit machine |
| [03-peer-apply/](03-peer-apply/) | Apply + receipt |
| [05-lineage-reverse/](05-lineage-reverse/) | Undo |
| [09-ci/](09-ci/) | Merge gates |
| [diagrams/](diagrams/) | Flows |
| [INDEX.md](INDEX.md) | Full map |
| [CONTRIBUTING.md](CONTRIBUTING.md) | How to change this playbook |

## Definition of done

1. Contract schemas + vectors executable  
2. Rust Host + Peer pass vectors  
3. Baseline Ops end-to-end  
4. Cap refuse → zero mutate effects  
5. Activity end → reverse or non-reversible mark  
6. Peer exposes **no** mint-root API  
