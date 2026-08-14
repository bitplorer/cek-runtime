# CEK Rust workspace

See [IMPLEMENTATION.md](IMPLEMENTATION.md), [HARDENING.md](HARDENING.md), [MATURITY.md](MATURITY.md), [INVARIANTS.md](INVARIANTS.md), [CHANGELOG-IMPL.md](CHANGELOG-IMPL.md).

```bash
cargo test --workspace
cargo run -p cek-cli -- demo
cargo run -p cek-cli -- vectors crates/cek-contract/vectors
./scripts/invariants.sh
./scripts/coverage.sh
```

31 CORE vectors. 87 unit/property tests. Host stores are traits (`OnceBackend` / `IdemBackend` / `LineageBackend`) with in-memory and JSON-file backends.
