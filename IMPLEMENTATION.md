# CEK reference implementation (Rust) — complete guide

Runnable Host + Peer aligned with [cek-framework](https://github.com/bitplorer/cek-framework) law and this repo’s design docs.

## Quick start

```bash
cargo test --workspace
cargo run -p cek-cli -- demo
cargo run -p cek-cli -- vectors crates/cek-contract/vectors
```

## Crates

| Crate | Responsibility |
|-------|----------------|
| `cek-contract` | Types, Baseline Ops, digests, vector load/check, law_generation |
| `cek-host-kernel` | mint, verify, sealed-args, once, idempotency, BoundAsk, project, lineage, receipts, reverse, **store traits + file backends** |
| `cek-peer-kernel` | profile, apply, receipt — **no mint** |
| `cek-ops-baseline` | In-memory kv |
| `cek-ops-ui` | In-memory UI targets (`morph` / `restore`) |
| `cek-cli` | Demo + vector runner |

TypeScript apply-only Peer: `ports/cek-peer-ts` (no mint).

## Pipeline (Host)

```text
Intent+Cap
  → action match
  → expiry
  → sealed-args bind (if present)
  → scopes (non-empty allow-list; empty = unrestricted)
  → idempotency lookup (before once; replay or conflict)
  → once ensure_available
  → BoundAsk
  → project Ops (kv.write → kv.set, …)
  → idempotency record
  → once commit (only after successful project)
  → lineage commit (if activity_id)
  → ResultMsg { kind, ops, digest }
```

Peer: `apply(Result)` → `Receipt` → optional `Host::report_receipt`.  
Reverse: `Host::end_activity` → Ops for Peer apply.

## Durable stores

Host talks only to traits. Swap backends without changing law.

| Trait | In-memory | File (JSON + atomic rename) |
|-------|-----------|------------------------------|
| `OnceBackend` | `OnceStore` | `FileOnceStore` (`once.json`) |
| `IdemBackend` | `IdemStore` | `FileIdemStore` (`idem.json`) |
| `LineageBackend` | `LineageStore` | `FileLineageStore` (`lineage.json`) |

```rust
let stores = FileStores::open("/var/lib/cek")?;
let host = Host::with_backends(
    Arc::new(stores.once),
    Arc::new(stores.idem),
    Arc::new(stores.lineage),
);
```

I/O or lock failure is **fail closed** (never skip once). Multi-process file locking and SQL backends remain out of scope.

## Guarantees

1. Cap refuse → zero Ops; world unchanged  
2. Once Cap second use → refuse  
3. Dispatch error does **not** burn a once-Cap  
4. Same idempotency key + same body → cached Result (even for once-Caps)  
5. Same idempotency key + different body → refuse  
6. Sealed-args tamper → refuse  
7. Digests start with `cek1:`  
8. Activity end → reverse Ops; landed preferred when receipt reported  
9. Commit after Activity ended → dispatch_error (no ghost cause)  
10. Peer has no mint API  

11. Scope deny → refuse, zero Ops; attenuate cannot widen  
12. `ui.morph` with snapshot → reverse `ui.dom.restore`; without → non_reversible  

See [HARDENING.md](HARDENING.md), [MATURITY.md](MATURITY.md), and [INVARIANTS.md](INVARIANTS.md).

## Layout

```text
Cargo.toml
HARDENING.md
MATURITY.md
IMPLEMENTATION.md
INVARIANTS.md
.github/workflows/cek.yml
scripts/invariants.sh
scripts/coverage.sh
crates/cek-contract/
crates/cek-host-kernel/   # store.rs + durable.rs + props.rs + fail_closed.rs
crates/cek-peer-kernel/
crates/cek-ops-ui/
ports/cek-peer-ts/         # apply-only; no mint
```

## Edge cases & testing

- [EDGE_CASES.md](EDGE_CASES.md) — closed and deferred edges
- [TESTING.md](TESTING.md) — unit, vectors, property-style, coverage
- [INVARIANTS.md](INVARIANTS.md) — executable never-regress map
- [HARDENING.md](HARDENING.md) — fail-closed rules
- [MATURITY.md](MATURITY.md) — stage checklist
