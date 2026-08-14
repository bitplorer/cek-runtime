# Edge cases — closed in reference implementation

| Edge | Risk | Closure |
|------|------|---------|
| Action mismatch | Ambient effects | Refuse, zero Ops |
| Cap expired (`now >= not_after`) | Stale authority | Refuse |
| Empty action / empty Cap id | Degenerate Cap | Refuse |
| Sealed-args tamper | Arg widening | Refuse |
| Once Cap second use | Double cause | Refuse |
| **Once Cap + dispatch error** | Burn Cap with no effect | **Commit once only after successful project** |
| Empty kv key | Nonsense Op | `dispatch_error` |
| Empty `activity_id` | Bad lineage key | `dispatch_error` |
| Idempotency same key + same body | Duplicate causes | Return **cached** Result |
| Idempotency same key + different body | Silent fork | Refuse conflict |
| Peer `FailBatch` | Partial apply of unknowns | **Abort rest** after first unknown |
| `authority_refusal` / `dispatch_error` on Peer | Spurious mutate | Apply is no-op |
| Double `end_activity` | Double reverse | Second call errors |
| Commit after Activity ended | Ghost causes | Lineage rejects |
| Partial apply | Undo wrong set | Reverse prefers **landed** when receipt annotated |
| Digest stability | Cache poison | `cek1:` SHA-256 canonical JSON |

## Open / deferred

| Edge | Status |
|------|--------|
| Concurrent once race under multi-Host | Mutex serializes in-process; multi-Host policy later |
| Cap crypto forge | Deferred (signatures) |
| Clock skew | Host clock is authority; Peer does not check expiry |
| Snapshot reverse for delete | Needs prior value store |
