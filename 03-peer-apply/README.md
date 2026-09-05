# 03 — Peer apply

Peer is a pure apply engine.

```text
fn apply(op: &Op, ctx: &mut ApplyCtx) -> Landed | Failed
```

## Rules

1. Apply Ops in listed order within one Result.  
2. Unknown Op: ignore | soft-fail | strict-reject per **profile**. Never crash kernel.  
3. Unknown optional meta: ignore on Baseline.  
4. Optional **apply receipt**: landed set + failed set. Not a Cap.  
5. Partial apply does not invent Host truth.  
6. No mint. No Cap keys. No lineage authority.  
7. **Apply budgets** (`max_nodes` default 256, `max_depth` default 16) and **single-flight**. Pre-walk `within_budget` before mutate. Over limit → apply nothing; Receipt `landed` is empty and `failed` holds every Op (same fail-closed shape as a full FailBatch, not a new Host `ResultKind`). Concurrent `apply` returns `None` and does not mutate. Flat `Vec<Op>` walks at depth 0; the depth gate is ready for nested children later.

## ApplyCtx

Only what the Peer needs to carry out Ops (handles to kv, log, device drivers under profile).  
Must not include Host mint handles or once-store authority.

## Crash / panic

Peer crash ≠ Host rewriting authorized set.  
Without receipt, reverse uses authorized set (may over-compensate; policy may mark uncertainty).
