#!/usr/bin/env bash
# Coverage + test inventory for the CEK reference workspace.
set -euo pipefail
cd "$(dirname "$0")/.."

echo "=== test inventory ==="
python3 - <<'PY'
import os, re
root = "crates"
tests = []
for dirpath, _, files in os.walk(root):
    if "/target/" in dirpath:
        continue
    for f in files:
        if not f.endswith(".rs"):
            continue
        path = os.path.join(dirpath, f)
        text = open(path, encoding="utf-8").read()
        for i, line in enumerate(text.splitlines(), 1):
            if re.match(r"\s*#\[test\]", line):
                tests.append((path, i))
print(f"{len(tests)} #[test] functions")
by = {}
for p, _ in tests:
    crate = p.split(os.sep)[1]
    by[crate] = by.get(crate, 0) + 1
for k in sorted(by):
    print(f"  {k:22} {by[k]:3d}")
vec = [f for f in os.listdir("crates/cek-contract/vectors") if f.endswith(".json")]
print(f"{len(vec)} vector fixtures")
if len(vec) < 41:
    raise SystemExit(f"expected >= 41 vectors, got {len(vec)}")
PY

echo "=== cargo test ==="
cargo test --workspace --offline

echo "=== vectors ==="
cargo run -p cek-cli --quiet -- vectors crates/cek-contract/vectors

if command -v node >/dev/null 2>&1; then
  echo "=== ts peer ==="
  node --experimental-strip-types --no-warnings \
    ports/cek-peer-ts/src/run-vectors.ts crates/cek-contract/vectors
fi

echo "=== invariants ==="
./scripts/invariants.sh

if command -v cargo-llvm-cov >/dev/null 2>&1; then
  echo "=== llvm-cov ==="
  cargo llvm-cov --workspace --summary-only
elif command -v cargo-tarpaulin >/dev/null 2>&1; then
  echo "=== tarpaulin ==="
  cargo tarpaulin --workspace --out Stdout
else
  echo "(llvm-cov not installed — inventory + tests are the gate)"
  echo "optional: cargo install cargo-llvm-cov && cargo llvm-cov --workspace --html --output-dir coverage"
fi
