# 06 — Profiles

## Baseline

Permanent interop profile: classic Ops only, receipts optional, idempotency optional.

## production-v1 (recommended claim bar)

| Feature | Required |
|---------|----------|
| Idempotency bind | yes (Host-side) |
| Apply receipts | yes (Peer → Host annotation) |
| Fail-closed once-store | yes |
| Baseline lowering still works | yes |

Baseline Peers remain valid without production-v1.  
Claims of “production CEK” in *this* framework should target production-v1 vectors.

Host projects Result Ops to this Peer's ability ∪ Baseline (LAW §11). Missing Manifest → Baseline-only.

## Unknown Op policy

| Policy | Behavior |
|--------|----------|
| tolerant | ignore / soft-fail |
| strict | reject path per policy; no kernel crash |

## Profile never grants Cap authority
