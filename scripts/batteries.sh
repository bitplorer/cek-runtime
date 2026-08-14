#!/usr/bin/env bash
# Stress / load / chaos / pen batteries (all ports).
set -euo pipefail
cd "$(dirname "$0")/.."

echo "==== rust batteries ===="
cargo test -p cek-host-kernel --offline batteries -- --test-threads=8

echo "==== python host batteries ===="
python3 ports/cek-host-py/test_batteries.py

echo "==== js peer batteries ===="
node ports/cek-peer-js/batteries.mjs

echo "batteries ok"
