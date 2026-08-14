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

45 CORE vectors. 114 unit/property tests. TS + WASM apply-only Peers. `kv.delete` prior reverse. Domain pack `ui.*`.
