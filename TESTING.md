# Testing — unit, vectors, properties, coverage

## Commands

```bash
cargo test --workspace
cargo run -p cek-cli -- vectors crates/cek-contract/vectors
cargo run -p cek-cli -- demo
./scripts/coverage.sh

# optional
cargo install cargo-llvm-cov
cargo llvm-cov --workspace --summary-only
cargo llvm-cov --workspace --html --output-dir coverage
```

## Layers

| Layer | Location | Proves |
|-------|----------|--------|
| **Unit** | `host::tests`, peer tests | Refuse, once, sealed, receipt, double-end, idem, no burn on dispatch error |
| **Vectors** | `crates/cek-contract/vectors/*.json` | Fixed CORE cases (CI gate) |
| **Property-style** | `props.rs`, `digest_props.rs` | Case tables: mismatch never effects; project; once; digest stable; expiry |
| **Coverage** | `scripts/coverage.sh` | Line coverage of Host/Peer/contract |

> External `proptest` is avoided on the reference toolchain (edition2024 dependency conflict). Property-style tests use deterministic case tables with the same invariants.

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
