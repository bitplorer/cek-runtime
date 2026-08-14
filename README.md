# CEK Runtime — Implementation Framework

**Not law.** Complements the conceptual charter at [bitplorer/cek-framework](https://github.com/bitplorer/cek-framework).

This repository documents **how to implement** CEK Host and Peer kernels, the interop contract, pipelines, profiles, isolation, and CI — without amending META/CORE.

| Layer | Lives in |
|-------|----------|
| Law (axioms, vocabulary, roles) | [cek-framework](https://github.com/bitplorer/cek-framework) |
| Runtime design + reference kernels | **this repo** |

## Canonical target (locked for this framework)

| Kernel | Language |
|--------|----------|
| Host | Rust only (`cek-host-kernel-rust`) |
| Peer | Rust only (`cek-peer-kernel-rust`) |

Python/TS/etc. may be **L7 callers** or later Peer *ports*. They are not Host kernels in this framework.

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

Law-level diagrams: [cek-framework/diagrams](https://github.com/bitplorer/cek-framework/tree/main/diagrams).

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
