# CEK Runtime

**Implement a CEK Host (authority) and Peer (apply surface) without ambient power.**

| | |
|--|--|
| **This repo** | Implementation playbook — contract, pipelines, isolation, CI, Rust crate shape |
| **Law** | [cek-framework](https://github.com/bitplorer/cek-framework) — Cap, Host, Peer, lineage, axioms |

| Status | Detail |
|--------|--------|
| Documented | Host/Peer split, submit order, reverse-first, vector merge gates |
| Next | Runnable crates + JSON vectors (layout is fixed so Cap cannot be skipped quietly) |

---

## This repo at a glance

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
         │ law meanings                        │ this playbook
         ▼                                     ▼
   cek-framework                         Host/Peer crates
```

| This repo **is** | This repo **is not** |
|------------------|----------------------|
| How to structure Host/Peer + contract | Replacement for [cek-framework](https://github.com/bitplorer/cek-framework) axioms |
| Topology, pipelines, CI bans | A required central “CEK cloud” service |
| Rust reference kernel design | Python/TS Host kernels (callers/ports only) |

| Piece | One line |
|-------|----------|
| **cek-contract** | Shared schemas + tests |
| **Host kernel** | Decide path inside Host process |
| **Peer kernel** | Apply path inside Peer process |
| **BoundAsk** | Token after verify; required for dispatch |
| **Receipt** | What Peer landed; guides reverse |
| **Baseline Ops** | Forever-interop data effects |

Full concept set: **[CONCEPTS.md](CONCEPTS.md)** · placement: **[TOPOLOGY.md](TOPOLOGY.md)**.

---

## Start here

| Doc | Why |
|-----|-----|
| Glance (above) | Whole repo in one box |
| **[CONCEPTS.md](CONCEPTS.md)** | Every implementation concept |
| **[TOPOLOGY.md](TOPOLOGY.md)** | Runtime vs kernel; wire |
| [00-contract/](00-contract/) | What `cek-contract` is |
| [cek-framework CONCEPTS](https://github.com/bitplorer/cek-framework/blob/main/CONCEPTS.md) | Law concepts |

---

## Runtime vs kernel

Kernels sit **inside** runtimes. No central broker kernel on the wire.

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

---

## Problems this solves

| You’re building | Risk without CEK runtime | What this design provides |
|-----------------|--------------------------|---------------------------|
| **Agent tools** (files, APIs, prod state) | Agent invents permission; weak revoke | Host verifies Cap; writes are Ops under lineage |
| **UI / DOM channel** (morph, patch, collab) | Client free-mutates; undo is guesswork | Peer applies `dom.*` Ops only; Host ends Activity → reverse/restore |
| **Device / robot commands** | Trusted firmware bypass | Same submit path; refuse → no effect Op |
| **Multi-version clients** | New Host breaks old Peer | Baseline Ops + profile projection |
| **Cancel / unload / revoke** | Half cleanup; fake rollback | Lineage + reverse plan (inverse, compensation, or honest mark) |
| **Authority PR review** | “Just this once” admin flags | No Cap-skip flags; Peer cannot mint; vectors block merge |

**DOM example:** Cap → Intent → `Result{Ops:[dom.morph]}` → Peer applies → morph **stays** until Activity end or Cap revoke → Host may emit restore/inverse Ops → Peer applies them. Completing a morph does **not** auto-undo.

---

## What you get

```text
Your app  --Intent + Cap-->  Host runtime (kernel inside)
                                |
                           Result + Ops
                                |
                              Peer runtime (kernel inside)
                                |
                           lineage / reverse on Host
```

**Host submit order (fail closed):**  
verify → once/idempotency → reload truth → dispatch → lineage → project → Result  
Cap refuse → **zero** mutate Ops.

---

## Which repo?

| Need | Open |
|------|------|
| Meanings, axioms, kill criteria | [cek-framework](https://github.com/bitplorer/cek-framework) |
| Host/Peer structure, topology, CI, crates | **This repo** |
| Product UI and business handlers | Your app — hold Caps, call `submit` |

**Kernels here:** Host and Peer in **Rust** only. Other languages = callers or later Peer ports.

---

## Design rule

**Contract is the sole interop product → Host is a Cap state machine → Peer is pure apply → reverse before rich domain Ops → vectors gate merge.**

## Navigate

| Path | Role |
|------|------|
| [CONCEPTS.md](CONCEPTS.md) | All implementation concepts at a glance |
| [TOPOLOGY.md](TOPOLOGY.md) | Runtime vs kernel; wire placement |
| [SCOPE.md](SCOPE.md) | Law vs this repo |
| [CONTRIBUTING.md](CONTRIBUTING.md) | How to change this playbook |
| [diagrams/](diagrams/) | Implementation flows |
| [00-contract/](00-contract/) | Schemas, vectors, Baseline |
| [01-kernels/](01-kernels/) | Host and Peer APIs |
| [02-host-pipeline/](02-host-pipeline/) | Submit machine |
| [03-peer-apply/](03-peer-apply/) | Apply + receipt |
| [05-lineage-reverse/](05-lineage-reverse/) | Undo story |
| [09-ci/](09-ci/) | Merge gates |
| [INDEX.md](INDEX.md) | Full map |

## Definition of done

1. Contract schemas + vector families executable  
2. Rust Host + Peer pass those vectors  
3. Baseline Ops end-to-end  
4. Cap refuse → zero mutate effects  
5. Activity end → reverse or non-reversible mark  
6. Peer crate exposes **no** mint-root API  
