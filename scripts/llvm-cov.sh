#!/usr/bin/env bash
# Produce llvm-cov HTML + text summary under coverage/.
# Uses cargo-llvm-cov when installed; otherwise rustc instrument-coverage.
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p coverage
HOST=$(rustc -vV | awk '/host:/{print $2}')
SYSROOT=$(rustc --print sysroot)
export LLVM_COV="${LLVM_COV:-$SYSROOT/lib/rustlib/$HOST/bin/llvm-cov}"
export LLVM_PROFDATA="${LLVM_PROFDATA:-$SYSROOT/lib/rustlib/$HOST/bin/llvm-profdata}"

if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
  echo "installing cargo-llvm-cov (one-time)..."
  cargo install cargo-llvm-cov --locked --quiet || cargo install cargo-llvm-cov --quiet
fi

echo "=== llvm-cov summary ==="
cargo llvm-cov --workspace --exclude cek-peer-wasm --summary-only --offline \
  | tee coverage/summary.txt

echo "=== llvm-cov html → coverage/html ==="
cargo llvm-cov --workspace --exclude cek-peer-wasm --html --output-dir coverage --offline
echo "wrote coverage/html/index.html"
