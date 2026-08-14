# CEK Runtime

**Build a CEK Host (authority) and Peer (apply surface) without inventing ambient power.**

This repo is the **implementation playbook**: contract, pipelines, isolation, CI gates, Rust-oriented crate shape.  
The **law** (meanings of Cap, Host, Peer, lineage…) lives in **[cek-framework](https://github.com/bitplorer/cek-framework)**.

| Status today | What’s fixed |
|--------------|--------------|
| Design + gates documented | Host/Peer split, submit order, reverse-first, vector merge rule |
| Runnable crates / JSON vectors | Next engineering step — layout is ready so code can’t “quietly” skip Cap |

---

## Real problems this is for

| You’re building… | Risk without a CEK runtime | What this design gives you |
|------------------|----------------------------|----------------------------|
| **Agent tools** that write files, call APIs, or change prod state | Agent invents permission; no clean revoke | Host verifies Cap; every write is an Op under lineage |
| **UI / DOM channel** (morph, patch, multiplayer surface) | Random client mutations; undo is guesswork | Peer only applies `dom.*` Ops; Host ends Activity → reverse/restore |
| **Device or robot commands** | Firmware “trusted path” bypasses policy | Same submit path; refuse = no motor/GPIO Op |
| **Multi-version clients** | New Host breaks old Peer | **Baseline** Ops + profile projection |
| **Cancel job / unload plugin / revoke access** | Half-applied cleanup, fake “rolled back” | Lineage + reverse plan (inverse, compensation, or honest mark) |
| **PR review on authority code** | “Just this once” admin flags | Kill ambient skips; Peer must not mint; vectors block merge |

**Example path (DOM):** Cap allows morph → Intent → Host Result `{ Ops: [dom.morph] }` → Peer applies → morph **stays** until Activity end/revoke → Host reverse may emit restore/inverse Ops → Peer applies those. Finishing a morph does **not** auto-undo.

---

## What this repo is (plain)

Two kernels, one contract:

```text
Your app  --Intent+Cap-->  Host (Rust)  --Result+Ops-->  Peer (Rust)
                              |
                         lineage / reverse
```

| Piece | Job |
|-------|-----|
| **Contract** | Schemas + tests (CORE 19 families) — interop product |
| **Host** | mint, verify, once/idempotency, dispatch, lineage, project, reverse |
| **Peer** | apply Ops in order; optional receipt; **never** mint root Caps |
| **Baseline Ops** | Small classic set (data only) so old Peers keep working |

**Submit order (Host, fail closed):**  
verify → once/idempotency → reload truth → dispatch → lineage → project → Result  
Refuse at Cap check → **zero** mutate Ops.

---

## When to use which repo

| I need… | Open |
|---------|------|
| What the words mean / axioms / “is this still CEK?” | [cek-framework](https://github.com/bitplorer/cek-framework) |
| How to structure Host/Peer, CI, crates, isolation | **this repo** |
| Product UI, business handlers | Your app (L7) — hold Caps, call `submit` |

---

## Canonical target

| Kernel | Language |
|--------|----------|
| Host | **Rust** only (`cek-host-kernel-rust`) |
| Peer | **Rust** only (`cek-peer-kernel-rust`) |

Python/TS/etc. = callers or later Peer ports — not Host kernels here.

---

## One line

**Contract is sole product → Host is Cap state machine → Peer is pure apply → reverse before rich Ops → vectors gate merge.**

## Start navigating

| Path | Role |
|------|------|
| [SCOPE.md](SCOPE.md) | Law vs this repo |
| [diagrams/](diagrams/) | Flows (optional Mermaid) |
| [00-contract/](00-contract/) | Schemas, vectors, Baseline |
| [02-host-pipeline/](02-host-pipeline/) | Submit machine |
| [03-peer-apply/](03-peer-apply/) | Apply + receipt |
| [05-lineage-reverse/](05-lineage-reverse/) | Undo story |
| [09-ci/](09-ci/) | Merge gates |
| [INDEX.md](INDEX.md) | Everything |

## Definition of done (reference runtime)

1. Contract schemas + CORE 19 vectors executable  
2. Rust Host + Peer green on those vectors  
3. Baseline Ops end-to-end  
4. Cap refuse → zero mutate effects  
5. Activity end → reverse or non-reversible mark  
6. Peer crate has **no** mint-root API  
