# Edge cases — closed in reference implementation

| Edge | Risk | Closure |
|------|------|---------|
| Action mismatch | Ambient effects | Refuse, zero Ops |
| Cap expired (`now >= not_after`) | Stale authority | Refuse |
| Empty action / empty Cap id | Degenerate Cap | Refuse |
| Sealed-args tamper | Arg widening | Refuse |
| Once Cap second use | Double cause | Refuse |
| **Once Cap + dispatch error** | Burn Cap with no effect | **Commit once only after successful dispatch** |
| Empty kv key | Nonsense Op | `dispatch_error` |
| Empty `activity_id` | Bad lineage key | `dispatch_error` |
| Idempotency same key + same body | Duplicate causes | Return **cached** Result |
| Idempotency same key + different body | Silent fork | Refuse conflict (**digest compared before once-ensure**) |
| Once-Cap + idempotent retry | Retry refused as consumed | Lookup **before** once-ensure; cached Result |
| Peer `FailBatch` | Partial apply of unknowns | **Abort rest** after first unknown |
| Peer `Skip` | Drop known Ops after unknown | Unknown skipped; later known Ops still apply |
| `authority_refusal` / `dispatch_error` on Peer | Spurious mutate | Apply is no-op |
| Double `end_activity` | Double reverse | Second call errors |
| Commit after Activity ended | Ghost causes | Lineage rejects **before** insert |
| Partial apply | Undo wrong set | Reverse prefers **landed** when receipt annotated |
| Digest stability | Cache poison | `cek1:` SHA-256 canonical JSON |
| `FailClosed::default()` vs serde | `once_store_down` false by derive | Manual `Default` matches serde (`true`) |
| Empty `idempotency_key` | Accidental global key | Refuse |
| Blank scope token | Unclear allow-list | Refuse |
| Attenuate widen | Derived Cap stronger than parent | `Host::attenuate` errors |
| Undeclared inject / over-limit | Ambient Context | `authority_refusal`, zero Ops (LAW §8) |
| isolate leak | Names/services cross Activities | `authority_refusal`, zero Ops; `limit` ≠ `isolate` |
| `ui.morph` with snapshot | Undo DOM | Inverse `ui.dom.restore` from Op payload |
| `ui.morph` without snapshot | Fake undo | `NonReversible` |
| Baseline Peer + `ui.dom.*` | Crash / mutate | Skip (unknown); lowering is optional `kv.set` |

## Open / deferred

| Edge | Status |
|------|--------|
| Concurrent once race under multi-thread | Mutex serializes; commit-after-project still races across hosts — multi-Host policy later |
| Multi-process file lock | One `File*Store` instance per directory; flock/SQL later |
| `kv.delete` with `prior` | Undo delete | Inverse `kv.set` from Op payload |
| `kv.delete` without `prior` | Fake undo | `NonReversible` |
| Cap HMAC missing / forged | Host has signing key | Refuse, zero Ops |
| Cap HMAC off | Host has no key | Legacy unsigned Caps still work |
| Ed25519 missing / forged / untrusted | Host has Ed25519 policy | Refuse, zero Ops |
| Unknown / blank law generation | Mixed contract ages | Refuse, zero Ops |
| Dual-speak previous gen | Migration window | `accept_generation` then project |
