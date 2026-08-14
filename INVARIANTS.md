# Invariants — executable, not just narrative

Law meanings live in [cek-framework](https://github.com/bitplorer/cek-framework).  
This file lists **what the reference runtime actually checks** and where.

## Never-regress (CI + property tables)

| # | Invariant | Proved by |
|---|-----------|-----------|
| 1 | Cap refuse → zero mutate Ops | `prop_action_mismatch_never_effects`, `prop_expired_never_effects`, vector checker |
| 2 | BoundAsk only after verify + once-ensure | `BoundAsk` fields are `pub(crate)`; no public constructor |
| 3 | Peer has no mint | CI + `invariants.sh` (Rust + TS) |
| 4 | Once commit only after successful project | `prop_once_not_burned_on_dispatch_error` |
| 5 | Idempotency before once-ensure | `prop_once_idempotent_retry`, `prop_idempotency_replay_and_conflict` |
| 6 | Same key + different body → refuse | `prop_idempotency_replay_and_conflict` |
| 7 | Landed-first reverse when receipt annotated | vector `receipt-landed-first-reverse` |
| 8 | Digests are `cek1:` + FIPS SHA-256 | `sha256_known_answers`, `prop_digest_stable_across_caps` |
| 9 | Fail closed on store down | `fail_closed` tests |
| 10 | Trace is not permission | `prop_trace_never_grants_authority` |
| 11 | Honest reverse | `prop_reverse_is_inverse_delete`, log.append → non_reversible |
| 12 | Concurrent once: exactly one `ok` | `concurrent_once_only_one_ok` |
| 13 | Scope deny / blank token → zero Ops | `prop_scope_deny_never_effects`, `empty-scope-token` |
| 14 | Attenuate cannot widen | `prop_attenuate_no_widen` |
| 15 | ui snapshot reverse / honest non-reversible | `prop_ui_snapshot_reverse`, `ui-morph-*` |
| 16 | kv.delete prior reverse / honest non-reversible | `prop_kv_delete_prior_reverse`, `kv-delete-*` |
| 17 | TS Peer has no mint | `invariants.sh` grep `ports/cek-peer-ts` |
| 18 | Empty idempotency key → refuse | vector `empty-idempotency-key` |
| 19 | Action ≠ Op | `actions::tests::actions_are_not_ops` |
| 20 | Cap HMAC missing/tamper → zero Ops | `prop_cap_hmac_never_effects_on_bad_sig`, `cap-sig-*` |
| 21 | Subject bind mismatch → zero Ops | `prop_subject_bind_never_effects_on_mismatch`, `subject-bind-*` |
| 22 | Ed25519 missing/tamper/wrong key → zero Ops | `ed25519_*` tests + `ed25519-*` vectors |
| 23 | Unknown/blank law generation → zero Ops | `law-gen-*` |

## Coverage

```bash
./scripts/coverage.sh
./scripts/invariants.sh
```
