# Testing — unit, vectors, properties, coverage

## How this tree is counted (SSoT)

Numbers below are from the tree at `e0d211bc7ca744b376184271f366d0abd69e6c9b` (current `main` HEAD). Do not cite 43, 57, 64, 71, 147, 152, or 190 — those are stale.

**72** JSON fixtures under `crates/cek-contract/vectors/`:

```bash
find crates/cek-contract/vectors -name '*.json' | wc -l
# 72
```

**16** distinct `family` values in those files (the JSON `family` field, not invented labels):

```bash
python3 -c "
import json, os, collections
d='crates/cek-contract/vectors'
c=collections.Counter()
for f in os.listdir(d):
    if f.endswith('.json'):
        c[json.load(open(os.path.join(d,f)))['family']] += 1
print(len(c), 'families;', sum(c.values()), 'files')
for k,v in sorted(c.items()):
    print(f'  {k} {v}')
"
# 16 families; 72 files
```

**261** `#[test]` attributes under `crates/` (same walk as `scripts/coverage.sh`: leading whitespace, then `#[test]`):

```bash
python3 - <<'PY'
import os, re
n = 0
for dirpath, _, files in os.walk("crates"):
    if "/target/" in dirpath:
        continue
    for f in files:
        if f.endswith(".rs"):
            text = open(os.path.join(dirpath, f), encoding="utf-8").read()
            n += sum(1 for line in text.splitlines() if re.match(r"\s*#\[test\]", line))
print(n, "#[test] functions")
PY
# 261 #[test] functions
```

`cargo test --workspace` may print a different “test” total (doctests, bins). The SSoT here is the attribute count, not that runner’s summary.

Port runners (TS / WASM / JS / PyO3 apply-only, Python Host) execute subsets of the same 72 JSON files. They are not extra fixtures.

## Commands

```bash
# Unit + property-style case tables
cargo test --workspace

# Conformance vectors
cargo run -p cek-cli -- vectors crates/cek-contract/vectors

# Demo
cargo run -p cek-cli -- demo

# Coverage inventory + tests + invariants
./scripts/coverage.sh

# Static never-regress greps
./scripts/invariants.sh

# Stress / load / chaos / pen
./scripts/batteries.sh

# llvm-cov (optional install)
cargo install cargo-llvm-cov
./scripts/llvm-cov.sh   # coverage/summary.txt + coverage/html
```

## Layers

| Layer | Location | Proves |
|-------|----------|--------|
| **Unit** | `host::tests`, peer, durable | Refuse, once, sealed, receipt, double-end, idem, file reopen |
| **Fail-closed** | `fail_closed.rs` | Store-down refuse; concurrent once (exactly one `ok`) |
| **Batteries** | `batteries.rs`, `ports/*/test_batteries*`, `scripts/batteries.sh` | Stress, load, chaos, pen — refuse stays zero-Ops |
| **Store contract** | `store::tests` | Memory backends satisfy trait contracts |
| **Vectors** | `crates/cek-contract/vectors/*.json` | **72** JSON files, **16** `family` values (see below) |
| **Property-style** | `props.rs`, `digest_props.rs`, `types_props.rs` | Deterministic tables (no `proptest` crate) |
| **SHA-256** | `digest::sha256_known_answers` | FIPS fixtures (`""`, `"abc"`, 56-byte) |
| **Coverage** | `scripts/coverage.sh` | Inventory + optional llvm-cov |
| **Invariants** | `scripts/invariants.sh`, [INVARIANTS.md](INVARIANTS.md) | Peer no-mint, BoundAsk private, refusal checker |

> **Note:** External `proptest` is avoided on the reference toolchain (edition2024 dep conflict).  
> Property-style tests use deterministic case tables with the same invariants.

## Vector families (this tree)

Law CORE/19 is the catalog in [cek-framework](https://github.com/bitplorer/cek-framework). This tree’s executable set is the 72 files above. Counts are `family` field occurrences:

| `family` | Files | What they lock |
|----------|------:|----------------|
| `cap_verify` | 19 | mismatch, expired, sealed, empty id/action, HMAC, Ed25519, subject bind, law generation |
| `single_use` | 2 | second use; not burned on dispatch error |
| `baseline_apply` | 5 | kv.set, apply lands, peer refusal / dispatch_error no-mutate |
| `baseline_lowering` | 6 | kv.delete, log.append, empty key, unknown action, missing message, ui morph lower |
| `unknown_ops` | 4 | skip continues, fail_batch aborts (incl. UI profile) |
| `unknown_meta` | 1 | extra JSON fields ignored |
| `lineage` | 3 | double end, commit after ended, empty activity_id |
| `reverse_on_end` | 4 | inverse delete; log.append non-reversible; kv.delete prior / no-prior |
| `apply_receipt` | 1 | landed-first reverse |
| `idempotent_submit` | 4 | replay, conflict, once-Cap retry, empty key |
| `trace` | 4 | not authority; groups; resume needs fresh Cap |
| `ui_domain` | 7 | morph project, snapshot reverse, no-snapshot non-reversible, empty target, Peer lands |
| `attenuation` | 3 | scope allow / deny / blank token |
| `context` | 4 | applied on submit; over-limit refuse; undeclared inject; isolate holds |
| `cap_revoke` | 2 | reverse and dead; honest non-reversible |
| `recovery_cap` | 3 | compensation usable / failure / revoke |

**7** files carry `peer_result` (no `intent`); **65** carry `intent` (no `peer_result`). TS / WASM / JS / PyO3 apply-only runners execute `peer_result` fixtures. Host-projected cases stay on the Rust / Python Host runners.

## Property invariants

1. ∀ mismatch(action, Cap.action) → refusal ∧ ops=∅  
2. ∀ valid kv.write → ops=[kv.set] with same key  
3. ∀ valid kv.delete / log.append → matching Baseline Op  
4. ∀ once Cap → second submit refuses  
5. ∀ identical projections → identical digests  
6. Sealed BTreeMap key order does not change digest  
7. ∀ expired Cap (`now >= not_after`) → refusal ∧ ops=∅  
8. ∀ kv.set under Activity → reverse is kv.delete of that key  
9. ∀ same idempotency key + same body → cached Result  
10. ∀ same key + different body → refuse  
11. ∀ sealed tamper → refuse; match → ok  
12. ∀ trace → never grants authority; grouping is query-only; resume still needs a fresh Cap  
13. ∀ once + dispatch miss → Cap not burned  
14. ∀ once + same idempotency key → retry is cached ok  
15. `ui.morph` + snapshot → restore reverse  
16. Scope deny → refuse ∧ ops=∅  
17. Attenuate cannot widen  
18. `kv.delete` + prior → `kv.set` reverse; without → non-reversible  

Current inventory: **261** `#[test]` + **72** vector fixtures + TS/WASM/JS/PyO3 apply-only + Python Host + batteries.

## Coverage targets

| Crate | Soft target |
|-------|-------------|
| cek-host-kernel | ≥ 80% lines |
| cek-contract | ≥ 70% lines |
| cek-peer-kernel | ≥ 70% lines |
| cek-ops-baseline | ≥ 70% lines |
| cek-ops-ui | ≥ 70% lines |

## CI

`.github/workflows/cek.yml` runs `scripts/invariants.sh`, `cargo test --workspace`, vectors, TS Peer, WASM Peer, PyO3 Peer (`scripts/run-pyo3-peer.sh`; needs `python3-dev`), and `scripts/coverage.sh`.
