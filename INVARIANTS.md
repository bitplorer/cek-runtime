# Invariants — executable, not just narrative

Law meanings live in [cek-framework](https://github.com/bitplorer/cek-framework).  
This file lists **what the reference runtime actually checks** and where.

## Never-regress (CI + property tables)

| # | Invariant | Proved by |
|---|-----------|-----------|
| 1 | Cap refuse → zero mutate Ops | `prop_action_mismatch_never_effects`, `prop_expired_never_effects`, `prop_every_refusal_is_effect_free`, vector checker |
| 2 | BoundAsk only after verify + once-ensure | `BoundAsk` fields are `pub(crate)`; no public constructor |
| 3 | Peer has no mint | CI grep `cek-peer-kernel` |
| 4 | Once commit only after successful project | `prop_once_not_burned_on_dispatch_error` |
| 5 | Idempotency before once-ensure | `prop_once_idempotent_retry`, `prop_idempotency_replay_and_conflict` |
| 6 | Same key + different body → refuse | `prop_idempotency_replay_and_conflict`, vector `idempotency-conflict` |
| 7 | Landed-first reverse when receipt annotated | vector `receipt-landed-first-reverse` |
| 8 | Digests are `cek1:` + FIPS SHA-256 | `sha256_known_answers`, `prop_digest_stable_across_caps` |
| 9 | Fail closed on store down | `fail_closed` tests (once / idem / lineage) |
| 10 | Trace is not permission | `prop_trace_never_grants_authority` |
| 11 | Honest reverse | `prop_reverse_is_inverse_delete`, `log_append` → `non_reversible` |
| 12 | Concurrent once: exactly one `ok` | `concurrent_once_only_one_ok` |

## Property tables (deterministic, no `proptest` crate)

| Table | Cases | Claim |
|-------|-------|-------|
| action mismatch | 6 keys × 3 values | never effects |
| kv.write project | 6 keys × 4 values | `kv.set` same key |
| kv.delete / log.append | keys / messages | Baseline lowering |
| once second use | 6 keys | refuse + empty Ops |
| digest stable | 6 keys | Cap id not in digest |
| expired | 6 keys | refuse |
| reverse | 6 keys | inverse delete |
| idempotency | 6 keys | replay + conflict |
| sealed-args | 6 keys | match ok / tamper refuse |
| trace | 4 strings | never grants authority |
| once + dispatch miss | 6 keys | not burned |
| once + idem retry | 6 keys | cached ok |
| SHA-256 | 3 FIPS fixtures | algorithm correctness |
| Peer refuse/dispatch | 3 messages | world unchanged |

## Coverage

Soft line targets (see [TESTING.md](TESTING.md)):

| Crate | Target |
|-------|--------|
| `cek-host-kernel` | ≥ 80% |
| `cek-contract` | ≥ 70% |
| `cek-peer-kernel` | ≥ 70% |
| `cek-ops-baseline` | ≥ 70% |

```bash
./scripts/coverage.sh
./scripts/invariants.sh
```
