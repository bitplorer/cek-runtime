#!/usr/bin/env bash
# Build the apply-only WASM Peer and run peer_result vectors.
set -euo pipefail
cd "$(dirname "$0")/.."
rustup target add wasm32-unknown-unknown >/dev/null
cargo build -p cek-peer-wasm --target wasm32-unknown-unknown --release --offline 2>/dev/null \
  || cargo build -p cek-peer-wasm --target wasm32-unknown-unknown --release
WASM=target/wasm32-unknown-unknown/release/cek_peer_wasm.wasm
node ports/cek-peer-wasm/run-vectors.mjs \
  "${1:-crates/cek-contract/vectors}" \
  "$WASM"
