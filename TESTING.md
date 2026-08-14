# Testing — unit, vectors, properties, coverage

## Commands

```bash
# Unit + property-style case tables
cargo test --workspace

# Conformance vectors
cargo run -p cek-cli -- vectors crates/cek-contract/vectors

# Demo
cargo run -p cek-cli -- demo

# Coverage inventory + tests + invariants
./scripts/coverage.sh

# Static never-regress greps
./scripts/invariants.sh

# llvm-cov (optional install)
cargo install cargo-llvm-cov
cargo llvm-cov --workspace --summary-only
cargo llvm-cov --workspace --html --output-dir coverage
```

## Layers

| Layer | Location | Proves |
|-------|----------|--------|
| **Unit** | `host::tests`, peer, durable | Refuse, once, sealed, receipt, double-end, idem, file reopen |
| **Fail-closed** | `fail_closed.rs` | Store-down refuse; concurrent once (exactly one `ok`) |
| **Store contract** | `store::tests` | Memory backends satisfy trait contracts |
| **Vectors** | `crates/cek-contract/vectors/*.json` | CORE families — **31** cases |
| **Property-style** | `props.rs`, `digest_props.rs`, `types_props.rs` | Deterministic tables (no `proptest` crate) |
| **SHA-256** | `digest::sha256_known_answers` | FIPS fixtures (`""`, `"abc"`, 56-byte) |
| **Coverage** | `scripts/coverage.sh` | Inventory + optional llvm-cov |
| **Invariants** | `scripts/invariants.sh`, [INVARIANTS.md](INVARIANTS.md) | Peer no-mint, BoundAsk private, refusal checker |

> **Note:** External `proptest` is avoided on the reference toolchain (edition2024 dep conflict).  
> Property-style tests use deterministic case tables with the same invariants.

## Vector families (vs CORE/19)

| Family | Vectors | CORE/19 row |
|--------|---------|-------------|
| `cap_verify` | mismatch, expired, expiry-at-boundary, sealed match/mismatch, empty action, empty cap id | Cap verify |
| `single_use` | second use, not burned on dispatch error | Single-use |
| `baseline_apply` | kv.set, apply lands, peer refusal / dispatch_error no-mutate | Baseline apply |
| `baseline_lowering` | kv.delete, log.append, empty key, unknown action, missing message | Baseline lowering |
| `unknown_ops` | skip continues, fail_batch aborts | Unknown Ops |
| `unknown_meta` | extra JSON fields ignored | Unknown meta |
| `lineage` | double end, commit after ended, empty activity_id | Lineage |
| `reverse_on_end` | inverse delete; log.append non-reversible | Reverse on end |
| `apply_receipt` | landed-first reverse | Apply receipt into lineage |
| `idempotent_submit` | replay, conflict, once-Cap retry | Idempotent submit |
| `trace` | shared trace does not grant authority | Trace |

Not yet as vectors: attenuation/scopes, Peer-no-mint (CI + `invariants.sh` grep).

## Property invariants

1. ∀ mismatch(action, Cap.action) → refusal ∧ ops=∅  
2. ∀ valid kv.write → ops=[kv.set] with same key  
3. ∀ valid kv.delete / log.append → matching Baseline Op  
4. ∀ once Cap → second submit refuses  
5. ∀ identical projections → identical digests  
6. Sealed BTreeMap key order does not change digest  
7. ∀ expired Cap (`now >= not_after`) → refusal ∧ ops=∅  
8. ∀ kv.set under Activity → reverse is kv.delete of that key  
9. ∀ same idempotency key + same body → cached Result  
10. ∀ same key + different body → refuse  
11. ∀ sealed tamper → refuse; match → ok  
12. ∀ trace → never grants authority  
13. ∀ once + dispatch miss → Cap not burned  
14. ∀ once + same idempotency key → retry is cached ok  
15. SHA-256 matches FIPS known answers  

## Coverage targets

| Crate | Soft target |
|-------|-------------|
| cek-host-kernel | ≥ 80% lines |
| cek-contract | ≥ 70% lines |
| cek-peer-kernel | ≥ 70% lines |
| cek-ops-baseline | ≥ 70% lines |

Current inventory (this tree): **87** `#[test]` functions + **31** vector fixtures.

## CI

`.github/workflows/cek.yml` runs `scripts/invariants.sh`, `cargo test --workspace`, vectors, and `scripts/coverage.sh`.
