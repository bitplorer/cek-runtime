# Hardening — CEK reference implementation

This document lists **fail-closed** and **determinism** rules enforced in code.

## Authority path

| Check | When | On failure |
|-------|------|------------|
| Action match | `intent.action == cap.action` | `authority_refusal`, zero Ops |
| Expiry | `now >= cap.not_after` | `authority_refusal`, zero Ops |
| Sealed-args bind | Cap has `sealed_args_bind` | Digest of Intent.args must equal bind; else refuse |
| Once consume | `cap.once` | Atomic insert before effects; second use refuses |
| Once store down | lock poison | Refuse (fail closed) |
| Idempotency | `idempotency_key` set | Same key + different digest → refuse |

## BoundAsk

`BoundAsk` is constructed **only** after the checks above succeed.  
There is no public constructor. Dispatch uses the bound Intent only.

## Results

| Kind | Mutate Ops allowed? | Digest |
|------|---------------------|--------|
| `ok` | Yes | `cek1:` SHA-256 over kind+ops+error |
| `authority_refusal` | **Never** | Always computed |
| `dispatch_error` | No (v0 empty) | Always computed |

Vector checker rejects `authority_refusal` with non-empty Ops.

## Lineage and reverse

1. Commit stores **authorized** Ops + inverse plan class.  
2. Peer receipt → `report_receipt` annotates **landed** Ops.  
3. `end_activity` prefers inverse of **landed** when annotated; else inverse from commit.  
4. `NonReversible` / `Compensation` entries are listed; never claimed clean.

## Peer

- No `mint` / `mint_root`.  
- Unknown Ops: profile policy (`Skip` or `FailBatch`).  
- Authority refusal Result → no world change.

## Digests

- Algorithm id prefix: `cek1:`  
- Canonical JSON via `serde_json` over ordered maps (`BTreeMap`).  
- Same inputs → same digest across platforms (pure SHA-256 in contract).

## Profiles

| Profile | Receipts | Notes |
|---------|----------|-------|
| `baseline` | Optional | Classic Ops |
| `production-v1` | Expected for landed-first reverse | Host still works without; maturity prefers receipts |

## Explicit non-goals (v0.1+)

Cap public-key crypto · durable DB backends · multi-Host federation policy · DOM snapshot store · network transport as kernel.
