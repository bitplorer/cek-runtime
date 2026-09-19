#!/usr/bin/env bash
# Build the apply-only PyO3 Peer and run peer_result vectors (+ cek apply).
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build -p cek-peer-pyo3 --features extension-module --release --offline 2>/dev/null \
  || cargo build -p cek-peer-pyo3 --features extension-module --release
cargo build -p cek-cli --release --offline 2>/dev/null \
  || cargo build -p cek-cli --release
SO=target/release/libcek_peer_pyo3.so
if [ ! -f "$SO" ]; then
  if [ -f target/release/cek_peer_pyo3.so ]; then
    SO=target/release/cek_peer_pyo3.so
  else
    echo "missing PyO3 cdylib (need python3-dev to build --features extension-module)" >&2
    exit 2
  fi
fi
export CEK_BIN="${CEK_BIN:-target/release/cek}"
python3 ports/cek-peer-pyo3/run-vectors.py \
  "${1:-crates/cek-contract/vectors}" \
  "$SO"
