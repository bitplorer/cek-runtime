# Testing — unit, vectors, properties, coverage

## Commands

```bash
# Unit + property-style case tables
cargo test --workspace

# Conformance vectors
cargo run -p cek-cli -- vectors crates/cek-contract/vectors

# Demo
cargo run -p cek-cli -- demo

# Coverage helper
./scripts/coverage.sh

# llvm-cov (optional install)
cargo install cargo-llvm-cov
cargo llvm-cov --workspace --summary-only
cargo llvm-cov --workspace --html --output-dir coverage
```

## Layers

| Layer | Location | Proves |
|-------|----------|--------|
| **Unit** | `host::tests`, peer tests, `durable::tests` | Refuse, once, sealed, receipt, double-end, idem, no burn on dispatch error, file reopen |
| **Store contract** | `store::tests` | Memory backends satisfy trait contracts |
| **Vectors** | `crates/cek-contract/vectors/*.json` | CORE families (CI gate) — 25 cases |
| **Property-style** | `props.rs`, `digest_props.rs` | Exhaustive tables: mismatch never effects; project; once; digest stable; expiry |
| **Coverage** | `scripts/coverage.sh` | Line coverage of Host/Peer/contract |

> **Note:** External `proptest` is avoided on the reference toolchain (edition2024 dep conflict).  
> Property-style tests use deterministic case tables with the same invariants.

## Vector families (vs CORE/19)

| Family | Vectors | CORE/19 row |
|--------|---------|-------------|
| `cap_verify` | action mismatch, expired, sealed match/mismatch, empty action, empty cap id | Cap verify |
| `single_use` | second use, not burned on dispatch error | Single-use |
| `baseline_apply` | kv.set project, apply lands, peer refusal no-mutate | Baseline apply |
| `baseline_lowering` | kv.delete, log.append, empty key, unknown action | Baseline lowering |
| `unknown_ops` | skip continues, fail_batch aborts | Unknown Ops |
| `lineage` | double end, commit after ended | Lineage |
| `reverse_on_end` | inverse delete | Reverse on end |
| `apply_receipt` | landed-first reverse | Apply receipt into lineage |
| `idempotent_submit` | replay, conflict, once-Cap retry | Idempotent submit |
| `trace` | shared trace does not grant authority | Trace |

Not yet as vectors: attenuation/scopes, unknown-meta ignore, Peer-no-mint (CI grep).

## Property invariants

1. ∀ mismatch(action, Cap.action) → refusal ∧ ops=∅  
2. ∀ valid kv.write → ops=[kv.set] with same key  
3. ∀ once Cap → second submit refuses  
4. ∀ identical projections → identical digests  
5. Sealed BTreeMap key order does not change digest  
6. ∀ expired Cap → refusal ∧ ops=∅  

## Coverage targets

| Crate | Soft target |
|-------|-------------|
| cek-host-kernel | ≥ 80% lines |
| cek-contract | ≥ 70% lines |
| cek-peer-kernel | ≥ 70% lines |

## CI

`.github/workflows/cek.yml` runs `cargo test --workspace`, vectors, and Peer no-mint grep.
