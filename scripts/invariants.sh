#!/usr/bin/env bash
# Static never-regress checks (no cargo required for most).
set -euo pipefail
cd "$(dirname "$0")/.."

fail() { echo "FAIL: $*"; exit 1; }
ok() { echo "ok  $*"; }

# 1. Peer must not mint (Rust + TS port).
if grep -REn 'pub[[:space:]]+fn[[:space:]]+mint|mint_root' crates/cek-peer-kernel >/dev/null; then
  fail "Peer must not expose mint"
fi
if [ -d ports/cek-peer-ts ]; then
  if grep -REn 'function mint|export function mint|mint_root' ports/cek-peer-ts >/dev/null; then
    fail "TS Peer must not expose mint"
  fi
  ok "TS Peer has no mint"
fi
ok "Peer has no mint"

# 2. BoundAsk has no public constructor.
if grep -REn 'pub[[:space:]]+(fn[[:space:]]+new|struct BoundAsk)' crates/cek-host-kernel/src/bound.rs | grep -v 'pub struct BoundAsk' >/dev/null; then
  :
fi
if grep -n 'pub fn new' crates/cek-host-kernel/src/bound.rs >/dev/null; then
  fail "BoundAsk must not have a public constructor"
fi
if ! grep -q 'pub(crate) intent' crates/cek-host-kernel/src/bound.rs; then
  fail "BoundAsk.intent must stay crate-private"
fi
ok "BoundAsk is privately constructed"

# 3. authority_refusal checker still rejects Ops.
if ! grep -q 'authority_refusal carried ops' crates/cek-contract/src/vectors.rs; then
  fail "vector checker must reject refusal+ops"
fi
ok "vector checker rejects refusal with Ops"

# 4. once commit is after project (comment + call site order in host.rs).
if ! grep -q 'Commit once-Cap only after successful project' crates/cek-host-kernel/src/host.rs; then
  fail "once-after-project comment missing"
fi
ok "once-after-project documented in Host"

# 5. digest prefix.
if ! grep -q 'cek1' crates/cek-contract/src/digest.rs; then
  fail "cek1 digest prefix missing"
fi
ok "cek1 digest prefix present"

# 6. vector count.
n=$(find crates/cek-contract/vectors -name '*.json' | wc -l)
if [ "$n" -lt 25 ]; then
  fail "expected >= 25 vectors, got $n"
fi
ok "vectors: $n"

echo "invariants ok"
