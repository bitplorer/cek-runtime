# CEK Runtime — Implementation Framework

**Not law.** How to **build** CEK Host and Peer kernels.  
Law lives in [cek-framework](https://github.com/bitplorer/cek-framework).

---

## For developers — what is this?

**cek-runtime** is the implementation playbook for CEK: contract shapes, Host/Peer design, pipelines, isolation, and CI — aimed at a **Rust** reference pair.

| Piece | Role |
|-------|------|
| **Contract** | Schemas + conformance vectors (the only interop product) |
| **Host kernel** | `cek-host-kernel-rust` — mint, verify, submit, lineage, reverse |
| **Peer kernel** | `cek-peer-kernel-rust` — apply Ops only; no mint |
| **Baseline Ops** | Classic data-only effects every Peer can aim at |

**This repo is design + gates today.** Runnable crates and JSON vectors are the next engineering step; the layout and rules are fixed so that work cannot invent ambient authority.

### What it does for you

1. Turns CEK law into a **submit machine** (verify → once/idempotency → dispatch → lineage → project → Result).  
2. Keeps **Peer pure** (apply + optional receipt).  
3. Forces **reverse** before rich domain features.  
4. Makes **vectors** the merge gate (red test = not CEK-aligned).

### Flow (implementation view)

```text
Your app holds a Cap
  → calls Host.submit(Intent, Cap)
  → Host refuses or returns Result{Ops}
  → Peer.apply(Result)
  → optional receipt back to Host
  → Activity end → Host reverse
```

### When is the runtime useful?

Use this when you are:

- implementing a **CEK-aligned** authority service (Host) and apply surface (Peer)  
- wiring **agents, UIs, or devices** that must only mutate via Ops  
- needing **fail-closed** Cap checks, once-consume, and undo  
- wanting a **stable interop** story (Baseline + profiles) across versions  
- reviewing a PR for “did we skip Cap / mint on Peer / silent undo?”

### When to use the other repo instead

| Need | Go here |
|------|--------|
| What the words mean / axioms / kill criteria | [cek-framework](https://github.com/bitplorer/cek-framework) |
| How to structure Host/Peer code, CI, crates | **this repo** |
| Your product handlers (L7) | Your app — call Host; don’t fork the law |

### Canonical target

| Kernel | Language |
|--------|----------|
| Host | Rust only |
| Peer | Rust only |

Other languages may be **callers** (hold Cap, call submit) or later Peer **ports**. They are not Host kernels in this framework.

---

## One line

**Contract is sole product → Host is Cap state machine → Peer is pure apply → reverse before rich Ops → vectors gate merge.**

## Diagrams

Start here: [diagrams/README.md](diagrams/README.md)

| View | Diagram |
|------|--------|
| System boundary | [01-system-boundary.mmd](diagrams/01-system-boundary.mmd) |
| Host submit pipeline | [02-submit-pipeline.mmd](diagrams/02-submit-pipeline.mmd) |
| End-to-end sequence | [09-happy-path.mmd](diagrams/09-happy-path.mmd) |
| Reverse | [05-reverse.mmd](diagrams/05-reverse.mmd) |
| Contract + CI | [06](diagrams/06-contract-product.mmd) · [11](diagrams/11-ci-gate.mmd) |

Plain-text summaries live in section READMEs. Law diagrams: [cek-framework/diagrams](https://github.com/bitplorer/cek-framework/tree/main/diagrams).

## Map

| Path | Role |
|------|------|
| [SCOPE.md](SCOPE.md) | What belongs here vs cek-framework |
| [INDEX.md](INDEX.md) | Full navigation |
| [diagrams/](diagrams/) | Implementation flow diagrams |
| [00-contract/](00-contract/) | Schemas, vectors, Baseline Ops, manifest |
| [01-kernels/](01-kernels/) | Host + Peer as only L1 implementations |
| [02-host-pipeline/](02-host-pipeline/) | Ordered submit machine |
| [03-peer-apply/](03-peer-apply/) | Apply, profile, receipt |
| [04-cap-machine/](04-cap-machine/) | Cap lifecycle in code |
| [05-lineage-reverse/](05-lineage-reverse/) | Lineage first; reverse plans |
| [06-profiles/](06-profiles/) | Baseline vs production profile |
| [07-isolation/](07-isolation/) | Process/WASM Peer boundary |
| [08-layout/](08-layout/) | Crate workspace |
| [09-ci/](09-ci/) | Vector merge gate; ambient bans |
| [10-ports/](10-ports/) | Later language/OS ports |
| [COMPLETENESS.md](COMPLETENESS.md) | Coverage audit |
| [CHOICES.md](CHOICES.md) | Why these implementation choices |

## Definition of done (reference runtime)

1. `cek-contract` schemas + CORE 19 vector families executable  
2. Rust Host + Rust Peer green on those vectors  
3. Baseline Ops apply end-to-end  
4. Cap refuse → zero mutate effects  
5. Activity end → reverse or non-reversible mark  
6. Peer crate has **no** mint-root API  
