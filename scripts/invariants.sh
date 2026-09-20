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
ok "Peer has no mint"
if [ -d ports/cek-peer-js ]; then
  if grep -REn 'function mint|export function mint|mint_root' ports/cek-peer-js >/dev/null; then
    fail "JS Peer must not expose mint"
  fi
  ok "JS Peer has no mint"
fi
if [ -d crates/cek-peer-rust ]; then
  if grep -REn 'pub[[:space:]]+fn[[:space:]]+mint|Host::mint|mint_root' crates/cek-peer-rust >/dev/null; then
    fail "Peer rust JSON door must not mint"
  fi
  ok "Peer rust JSON door has no mint"
fi
if [ -d crates/cek-peer-wasm ]; then
  if grep -REn 'pub[[:space:]]+fn[[:space:]]+mint|Host::mint|mint_root' crates/cek-peer-wasm >/dev/null; then
    fail "WASM Peer crate must not mint"
  fi
  ok "WASM Peer crate has no mint"
fi
if [ -d crates/cek-peer-pyo3 ]; then
  if grep -REn 'pub[[:space:]]+fn[[:space:]]+mint|Host::mint|mint_root' crates/cek-peer-pyo3 >/dev/null; then
    fail "PyO3 Peer crate must not mint"
  fi
  ok "PyO3 Peer crate has no mint"
fi
if [ -d ports/cek-peer-pyo3 ]; then
  if grep -REn 'def mint|mint_root|Host::mint' ports/cek-peer-pyo3 >/dev/null; then
    fail "PyO3 Peer port must not mint"
  fi
  ok "PyO3 Peer port has no mint"
fi

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

# 4. once commit is after dispatch (comment + call site order in host.rs).
if ! grep -q 'Consume once only after successful dispatch' crates/cek-host-kernel/src/host.rs; then
  fail "once-after-dispatch comment missing"
fi
ok "once-after-dispatch documented in Host"
if grep -q 'consume_once' 02-host-pipeline/README.md; then
  fail "host pipeline must not describe a single consume_once step (LAW §12 two-phase)"
fi
if ! grep -q 'once.commit' 02-host-pipeline/README.md; then
  fail "host pipeline must document once.commit after dispatch (LAW §12)"
fi
if ! grep -q 'ensure_available' 02-host-pipeline/README.md; then
  fail "host pipeline must document ensure_available before dispatch (LAW §12)"
fi
ok "host pipeline documents two-phase once (LAW §12)"

# 8. LAW §4: lineage recorded before project in dispatch_and_finish.
if ! grep -q 'LAW §4 step 4: Record lineage' crates/cek-host-kernel/src/host.rs; then
  fail "LAW §4 lineage-before-project comment missing"
fi
ok "LAW §4 lineage-before-project documented"

# 5. digest prefix.
if ! grep -q 'cek1' crates/cek-contract/src/digest.rs; then
  fail "cek1 digest prefix missing"
fi
ok "cek1 digest prefix present"

# 6. vector count.
n=$(find crates/cek-contract/vectors -name '*.json' | wc -l)
if [ "$n" -lt 57 ]; then
  fail "expected >= 57 vectors, got $n"
fi
ok "vectors: $n"

# 7. action / Op split documented (ui.morph is not ui.dom.morph).
if ! grep -q 'Actions are never applied' crates/cek-contract/src/actions.rs; then
  fail "action vs Op split missing"
fi
ok "action vs Op split documented"

# 9. No sibling shared JSON package (hops own their wire).
if [ -d crates/cek-peer-json ]; then
  fail "cek-peer-json must not exist (hops own JSON; kernel owns typed apply)"
fi
ok "no cek-peer-json crate"

# 10. Peer hop graph: no sibling→sibling (wasm ↛ rust, rust ↛ wasm).
# #10 moved JSON into rust and pointed wasm at it (wrong owner). Restored.
wasm_toml=crates/cek-peer-wasm/Cargo.toml
rust_toml=crates/cek-peer-rust/Cargo.toml
if awk '
  $0 ~ /^\[/ { d = ($0 == "[dependencies]" || $0 == "[dev-dependencies]") }
  d && $0 ~ /^cek-peer-rust[[:space:]]*=/ { found=1 }
  END { exit !found }
' "$wasm_toml"; then
  fail "cek-peer-wasm must not depend on cek-peer-rust (sibling→sibling; #10 wrong-owner)"
fi
ok "cek-peer-wasm does not depend on cek-peer-rust"
if awk '
  $0 ~ /^\[/ { d = ($0 == "[dependencies]" || $0 == "[dev-dependencies]") }
  d && $0 ~ /^cek-peer-kernel[[:space:]]*=/ { found=1 }
  END { exit !found }
' "$wasm_toml"; then
  :
else
  fail "cek-peer-wasm must depend on cek-peer-kernel"
fi
ok "cek-peer-wasm depends on cek-peer-kernel"
if awk '
  $0 ~ /^\[/ { d = ($0 == "[dependencies]" || $0 == "[dev-dependencies]") }
  d && $0 ~ /^cek-peer-wasm[[:space:]]*=/ { found=1 }
  END { exit !found }
' "$rust_toml"; then
  fail "cek-peer-rust must not depend on cek-peer-wasm (sibling→sibling)"
fi
ok "cek-peer-rust does not depend on cek-peer-wasm"

# 11. Twin JSON wire field freeze (hops own serde; no shared JSON crate).
rust_lib=crates/cek-peer-rust/src/lib.rs
wasm_lib=crates/cek-peer-wasm/src/lib.rs
for struct_name in ApplyRequest ApplyResponse; do
  rust_fields=$(awk -v n="$struct_name" '
    $0 ~ "pub struct " n { p=1 }
    p && /pub [a-z_]+[[:space:]]*:/ {
      gsub(/^.*pub[[:space:]]+/, "")
      gsub(/[[:space:]]*:.*/, "")
      print
    }
    p && /^}/ { exit }
  ' "$rust_lib")
  wasm_fields=$(awk -v n="$struct_name" '
    $0 ~ "pub struct " n { p=1 }
    p && /pub [a-z_]+[[:space:]]*:/ {
      gsub(/^.*pub[[:space:]]+/, "")
      gsub(/[[:space:]]*:.*/, "")
      print
    }
    p && /^}/ { exit }
  ' "$wasm_lib")
  if [ "$rust_fields" != "$wasm_fields" ]; then
    fail "cek-peer-rust and cek-peer-wasm $struct_name fields drifted"
  fi
done
ok "rust/wasm ApplyRequest+ApplyResponse fields match"

# 12. One-shot apply_world must not coerce Peer::apply None to empty success.
if awk '
  /^#\[cfg\(test\)\]/ { exit }
  /unwrap_or\(Receipt/ { found=1 }
  END { exit !found }
' crates/cek-peer-kernel/src/apply.rs; then
  fail "apply_world must not map Peer::apply None via unwrap_or(Receipt"
fi
ok "apply_world does not coerce None to empty receipt"

# 13. Host runtime: kernel only; no peer coupling; no host-pyo3.
if [ -d crates/cek-host-pyo3 ]; then
  fail "cek-host-pyo3 must not exist (published Cap machine stays pip install cek-host)"
fi
ok "no cek-host-pyo3 crate"
host_rust_toml=crates/cek-host-rust/Cargo.toml
if awk '
  $0 ~ /^\[/ { d = ($0 == "[dependencies]" || $0 == "[dev-dependencies]") }
  d && $0 ~ /^cek-host-kernel[[:space:]]*=/ { found=1 }
  END { exit !found }
' "$host_rust_toml"; then
  :
else
  fail "cek-host-rust must depend on cek-host-kernel"
fi
ok "cek-host-rust depends on cek-host-kernel"
if awk '
  $0 ~ /^\[/ { d = ($0 == "[dependencies]" || $0 == "[dev-dependencies]") }
  d && $0 ~ /^cek-peer-(kernel|rust|wasm|pyo3)[[:space:]]*=/ { found=1 }
  END { exit !found }
' "$host_rust_toml"; then
  fail "cek-host-rust must not depend on cek-peer-* crates"
fi
ok "cek-host-rust does not depend on peer crates"
if ! grep -q 'cek_host_rust::' crates/cek-cli/src/main.rs; then
  fail "cek host-json must hop onto cek-host-rust"
fi
if awk '
  /^fn run_host_json/ { p=1 }
  p && /Host::/ { found=1 }
  p && /^fn / && !/^fn run_host_json/ { exit }
  END { exit !found }
' crates/cek-cli/src/main.rs; then
  fail "cek host-json must not construct the kernel Host itself"
fi
ok "cek host-json hops onto cek-host-rust"

echo "invariants ok"
