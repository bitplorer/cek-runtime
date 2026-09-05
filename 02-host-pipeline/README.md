# 02 — Host pipeline

`submit` is a thin orchestrator over an ordered machine. Fail closed.

Once is **two-phase** (LAW §12; Host pipeline [GUIDE §4](../GUIDE.md#4-host-pipeline-fail-closed)):
`ensure_available` before dispatch (check only — do not burn);
`commit` only after successful dispatch. A dispatch miss or refuse
returns early and leaves the once-Cap unburned.

```text
1. verify_and_bind(intent, cap) -> BoundAsk | Refuse
   # includes once.ensure_available — LAW §12 check only, no burn
2. check_idempotency(BoundAsk) -> Ok | Refuse
   # submit looks up idempotency *before* once-ensure so a once-Cap
   # retry returns the cached Result instead of refusing
3. reload_truth(store)                    # never trust caller world-state
4. dispatch(BoundAsk) -> Outcome          # miss → early return, Cap unburned
5. once.commit                            # LAW §12: only after successful dispatch
6. commit_lineage(authorized_ops, reverse_plan)  # when required
7. project(Outcome, peer_profile) -> Ops
8. Result { ops | error }
```

LAW §4 order held: Verify → Consume/ensure → Dispatch → once.commit →
lineage → project → Result.

## Rules

| Stage | On failure |
|-------|------------|
| Verify | Authority refusal; **no** mutate Ops; **no** lineage cause |
| Context mediate (LAW §8) | Undeclared inject / over-limit / isolate leak → `authority_refusal`, zero Ops |
| Once / idempotency store down | Refuse |
| Duplicate idempotency bind | Return prior Result; no second cause |
| Dispatch policy deny after verify | Error Result; **no** `once.commit` (Cap unburned); no silent partial world change |
| Required lineage write fail | Refuse (fail closed) |

## Projection

Host projects Result Ops to `peer.apply_set ∪ Baseline` (LAW §11).  
Missing Manifest → assume Baseline-only Peer. Manifest never grants Cap.  
Empty Ops on success is allowed (pure decision).

## Bootstrap

Host-private, minimal, documented. Not a Peer API.  
Distinguish bootstrap-origin mint in audit/lineage when possible.
