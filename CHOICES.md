# Implementation choices

Rationale for decisions that are **not** CEK law.

| Choice | Rejected | Why |
|--------|----------|-----|
| Host + Peer both in **Rust** | Python Host; TS Peer as primary | One fail-closed reference; no dual-Host semantic drift |
| Contract crate as sole interop product | Docs-only correctness | CORE 19 requires executable vectors |
| Cap as state machine API | Single `submit` blob that can skip steps | Unrepresentable success-after-failed-verify |
| Ops as pure data + pure `apply` fn | Closures / eval Ops | Baseline ban on code emission |
| Reverse before rich domain Ops | Domain catalog first | Accountability loop must work on day one |
| Production profile = receipts + idempotency | Baseline-only forever in prod claims | Safer retries and honest reverse |
| Peer may be process/WASM-isolated | Always same address space | Forces boundary early |
| No second official Host language in this framework | Rust + Python Hosts | User decision: single Host kernel language |
| Crate names `cek-host-kernel-rust` / `cek-peer-kernel-rust` | Generic `host`/`peer` | Role + language explicit |
| L7 any language | Forcing all apps in Rust | Callers hold Caps; kernels stay Rust |
