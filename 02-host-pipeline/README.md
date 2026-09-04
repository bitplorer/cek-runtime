# 02 — Host pipeline

`submit` is a thin orchestrator over an ordered machine. Fail closed.

```text
1. verify_and_bind(intent, cap) -> BoundAsk | Refuse
2. consume_once / check_idempotency(BoundAsk) -> Ok | Refuse
3. reload_truth(store)                    # never trust caller world-state
4. dispatch(BoundAsk) -> Outcome
5. commit_lineage(authorized_ops, reverse_plan)  # when required
6. project(Outcome, peer_profile) -> Ops
7. Result { ops | error }
```

## Rules

| Stage | On failure |
|-------|------------|
| Verify | Authority refusal; **no** mutate Ops; **no** lineage cause |
| Once / idempotency store down | Refuse |
| Duplicate idempotency bind | Return prior Result; no second cause |
| Dispatch policy deny after verify | Error Result; no silent partial world change |
| Required lineage write fail | Refuse (fail closed) |

## Projection

Host projects Result Ops to `peer.apply_set ∪ Baseline` (LAW §11).  
Missing Manifest → assume Baseline-only Peer. Manifest never grants Cap.  
Empty Ops on success is allowed (pure decision).

## Bootstrap

Host-private, minimal, documented. Not a Peer API.  
Distinguish bootstrap-origin mint in audit/lineage when possible.
