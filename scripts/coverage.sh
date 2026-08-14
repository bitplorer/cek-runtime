#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
echo "=== cargo test ==="
cargo test --workspace
echo "=== vectors ==="
cargo run -p cek-cli --quiet -- vectors crates/cek-contract/vectors
if command -v cargo-llvm-cov >/dev/null 2>&1; then
  echo "=== llvm-cov ==="
  cargo llvm-cov --workspace --summary-only
elif command -v cargo-tarpaulin >/dev/null 2>&1; then
  echo "=== tarpaulin ==="
  cargo tarpaulin --workspace --out Stdout
else
  echo "Install coverage: cargo install cargo-llvm-cov"
  echo "Then: cargo llvm-cov --workspace --html --output-dir coverage"
fi
