# 05 — Lineage and reverse

**Implement reverse before rich domain Ops.**

## Lineage entry (minimum)

```text
cause_id
cap_id
activity_id?
trace_id?          # correlation only — Host::for_trace / LineageBackend::for_trace
action
sealed_ref
authorized_ops     # Host authorized set
reverse_plan       # inverse | compensation | non_reversible class
```

`trace?` is optional association. Query groups related Intents. Trace is never permission, never execute, never undo. `end_activity` / `revoke` stay lineage- and Cap-scoped (not reverse-by-trace). Resume still needs a fresh Cap.

## Reverse plan classes

| Class | Behavior |
|-------|----------|
| Inverse | Direct undo Ops where possible |
| Compensation | Submit Intents under a **recovery Cap** (LAW §13; `Host::mint_recovery`) |
| Non-reversible | Explicit mark + audit; never report clean reverse |

## Rule (implementation strictness)

A mutate Op should not land without a reverse-plan **class** assigned at lineage commit.  
Observe-only Ops are an explicit class (or default mutate until proven observe).

## Order

Reverse prefers **landed** set if receipt exists; else **authorized** set.  
Causal order: typically reverse order of causes unless compensation graph says otherwise.

## Activity end / Cap revoke

Must run reverse (LAW §9). `end_activity` is Activity-scoped; `Host::revoke` is Cap-scoped (`for_cap`). Failed reverse → NonReversible mark, not silent success. Compensation submits ordinary Intents under a Recovery Cap (`Host::mint_recovery`, LAW §13); failure is NonReversible.
