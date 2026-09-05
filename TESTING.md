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

# Stress / load / chaos / pen
./scripts/batteries.sh
```

## Layers

| Layer | Location | Proves |
|-------|----------|--------|
| **Unit** | `host::tests`, peer, durable | Refuse, once, sealed, receipt, double-end, idem, file reopen |
| **Fail-closed** | `fail_closed.rs` | Store-down refuse; concurrent once (exactly one `ok`) |
| **Batteries** | `batteries.rs`, `ports/*/test_batteries*`, `scripts/batteries.sh` | Stress, load, chaos, pen — refuse stays zero-Ops |
| **Store contract** | `store::tests` | Memory backends satisfy trait contracts |
| **Vectors** | `crates/cek-contract/vectors/*.json` | CORE families — **64** cases |
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
| **Vectors** | `crates/cek-contract/vectors/*.json` | CORE families — **43** cases |
| **Property-style** | `props.rs`, `digest_props.rs`, `types_props.rs` | Deterministic tables (no `proptest` crate) |
| **SHA-256** | `digest::sha256_known_answers` | FIPS fixtures (`""`, `"abc"`, 56-byte) |
| **Coverage** | `scripts/coverage.sh` | Inventory + optional llvm-cov |
| **Invariants** | `scripts/invariants.sh`, [INVARIANTS.md](INVARIANTS.md) | Peer no-mint, BoundAsk private, refusal checker |

> **Note:** External `proptest` is avoided on the reference toolchain (edition2024 dep conflict).  
> Property-style tests use deterministic case tables with the same invariants.

## Vector families (vs CORE/19)

| Family | Vectors | CORE/19 row |
|--------|---------|-------------|
| `cap_verify` | mismatch, expired, sealed, empty id/action, HMAC, **subject bind** | Cap verify |
| `single_use` | second use, not burned on dispatch error | Single-use |
| `baseline_apply` | kv.set, apply lands, peer refusal / dispatch_error no-mutate | Baseline apply |
| `baseline_lowering` | kv.delete, log.append, empty key, unknown action, missing message | Baseline lowering |
| `unknown_ops` | skip continues, fail_batch aborts | Unknown Ops |
| `unknown_meta` | extra JSON fields ignored | Unknown meta |
| `lineage` | double end, commit after ended, empty activity_id | Lineage |
| `reverse_on_end` | inverse delete; log.append non-reversible; **kv.delete prior / no-prior** | Reverse on end |
| `apply_receipt` | landed-first reverse | Apply receipt into lineage |
| `idempotent_submit` | replay, conflict, once-Cap retry | Idempotent submit |
| `trace` | shared trace does not grant authority; groups related Intents; resume still needs a fresh Cap | Trace |

| `ui_domain` | morph project, snapshot reverse, no-snapshot non-reversible, empty target, Peer lands | Domain pack |
| `attenuation` | scope allow / deny / blank token | Attenuation |
| `context` | applied on submit; over-limit refuse; undeclared inject; isolate holds | Activity / Context (LAW §8) |

TS apply-only runner executes `peer_result` fixtures (same JSON). Host-projected cases stay on the Rust runner.

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
12. ∀ trace → never grants authority; grouping is query-only; resume still needs a fresh Cap  
13. ∀ once + dispatch miss → Cap not burned  
14. ∀ once + same idempotency key → retry is cached ok  
15. `ui.morph` + snapshot → restore reverse  
16. Scope deny → refuse ∧ ops=∅  
17. Attenuate cannot widen  

18. `kv.delete` + prior → `kv.set` reverse; without → non-reversible  

Current inventory: **152** `#[test]` + **71** vector fixtures + TS/WASM/JS apply-only + Python Host + batteries.

```bash
./scripts/llvm-cov.sh   # coverage/summary.txt + coverage/html
```

## Coverage targets

| Crate | Soft target |
|-------|-------------|
| cek-host-kernel | ≥ 80% lines |
| cek-contract | ≥ 70% lines |
| cek-peer-kernel | ≥ 70% lines |
| cek-ops-baseline | ≥ 70% lines |
| cek-ops-ui | ≥ 70% lines |

## CI

`.github/workflows/cek.yml` runs `scripts/invariants.sh`, `cargo test --workspace`, vectors, TS Peer, and `scripts/coverage.sh`.

