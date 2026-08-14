# CEK Rust workspace

See [IMPLEMENTATION.md](IMPLEMENTATION.md), [HARDENING.md](HARDENING.md), [MATURITY.md](MATURITY.md), [INVARIANTS.md](INVARIANTS.md), [CHANGELOG-IMPL.md](CHANGELOG-IMPL.md).

```bash
cargo test --workspace
cargo run -p cek-cli -- demo
cargo run -p cek-cli -- vectors crates/cek-contract/vectors
./scripts/invariants.sh
node --experimental-strip-types --no-warnings \
  ports/cek-peer-ts/src/run-vectors.ts crates/cek-contract/vectors
bash scripts/run-wasm-peer.sh
```

51 CORE vectors. 123 tests. Subject bind. Optional Cap HMAC. llvm-cov HTML.
