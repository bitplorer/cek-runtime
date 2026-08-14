# Hardening — CEK reference implementation

This document lists **fail-closed** and **determinism** rules enforced in code.

## Authority path

| Check | When | On failure |
|-------|------|------------|
| Action match | `intent.action == cap.action` | `authority_refusal`, zero Ops |
| Expiry | `now >= cap.not_after` | `authority_refusal`, zero Ops |
| Sealed-args bind | Cap has `sealed_args_bind` | Digest of Intent.args must equal bind; else refuse |
| Scopes | `cap.scopes` non-empty | Resource must match allow-list; blank token → refuse |
| Attenuation | `Host::attenuate` | Child scopes must be a narrowing; widen refused |
| Idempotency | `idempotency_key` set | **Before** once-ensure. Empty key → refuse. Same digest → cached Result; different → refuse |
| Once available | `cap.once` | Ensure not consumed **before** project |
| Once commit | after successful project | Mark consumed; dispatch error does **not** burn |
| Once second use | already consumed | Refuse (unless idempotent replay already returned) |
| Once store down | lock poison / I/O | Refuse (fail closed) |
| Idem store down | lock poison / I/O | Refuse (fail closed) |

## BoundAsk

`BoundAsk` is constructed **only** after Cap verify + once-ensure succeed.  
There is no public constructor. Dispatch uses the bound Intent only.  
Idempotent replay returns a cached Result **without** constructing a new BoundAsk.

## Results

| Kind | Mutate Ops allowed? | Digest |
|------|---------------------|--------|
| `ok` | Yes | `cek1:` SHA-256 over kind+ops+error |
| `authority_refusal` | **Never** | Always computed |
| `dispatch_error` | No (v0 empty) | Always computed |

Vector checker rejects `authority_refusal` with non-empty Ops.

## Lineage and reverse

1. Commit stores **authorized** Ops + inverse plan class.  
2. Commit onto an ended Activity is rejected **before** insert.  
3. Peer receipt → `report_receipt` annotates **landed** Ops.  
4. `end_activity` prefers inverse of **landed** when annotated; else inverse from commit.  
5. `NonReversible` / `Compensation` entries are listed; never claimed clean.  
6. `ui.dom.morph` reverse is `ui.dom.restore` **only** when `snapshot` is on the Op; otherwise mark non-reversible.  
7. `kv.delete` reverse is `kv.set` **only** when `prior` is on the Op; otherwise mark non-reversible.

## Stores

| Trait | Must |
|-------|------|
| `OnceBackend` | `ensure_available` does not burn; `commit` only after project; down → refuse |
| `IdemBackend` | same digest replay; different digest refuse; down → refuse |
| `LineageBackend` | no commit after end; persist landed; down → error |

File backends write JSON via temp + rename. They are not a multi-process lock.

## Peer

- No `mint` / `mint_root` (Rust crate, `ports/cek-peer-ts`, and `cek-peer-wasm`).  
- Unknown Ops: profile policy (`Skip` or `FailBatch`).  
- `Peer::with_ui()` adds `ui.dom.morph` / `ui.dom.restore` to apply-set.  
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

## Manifest

`FailClosed::default()` and serde both set `once_store_down: true`. A derive-`Default` would have been `false` (unsafe skip).

## Explicit non-goals (v0.1+)

Cap public-key crypto · SQL/DB backends · multi-Host federation policy · DOM snapshot store · network transport as kernel.

