# CEK reference implementation (Rust)

**Read [START.md](START.md) first.** Walkthrough: **[GUIDE.md](GUIDE.md)**. Topology: [TOPOLOGY.md](TOPOLOGY.md).


Runnable Host + Peer aligned with [cek-framework](https://github.com/bitplorer/cek-framework) law.

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
| `cek-host-kernel` | mint, verify, sealed-args, once, idempotency, BoundAsk, dispatch, lineage, project, receipts, reverse, **store traits + file backends** |
| `cek-peer-kernel` | profile, apply, receipt — **no mint** |
| `cek-ops-baseline` | Peer **driver**: in-memory kv — [DRIVERS.md](DRIVERS.md) |
| `cek-ops-ui` | Peer **driver**: UI map + `DomTree` — [DRIVERS.md](DRIVERS.md) |
| `cek-cli` | Demo + vector runner |

TypeScript apply-only Peer: `ports/cek-peer-ts`.  
JavaScript Peer **runtime** (apply + DomTree): `ports/cek-peer-js`.  
Python Host **runtime (published):** `pip install cek-host`. `ports/cek-host-py` is a contract-vector sketch, not a second published Host.  
WASM apply-only Peer: `crates/cek-peer-wasm` + `ports/cek-peer-wasm`.

## Pipeline (Host)

```text
Intent+Cap
  → action match
  → expiry
  → sealed-args bind (if present)
  → scopes
  → subject bind (if Cap.subject set)
  → Context mediate (LAW §8: inject / limit / isolate; fail closed)
  → idempotency lookup (before once; replay or conflict)
  → once ensure_available
  → BoundAsk
  → dispatch (kv.write → authorized kv.set, …)
  → idempotency record
  → once commit (only after successful dispatch)
  → lineage commit (if activity_id; persist optional Intent.trace)
  → project Ops onto Result
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
11. Scope deny / blank scope token → refuse, zero Ops; attenuate cannot widen  
12. `ui.morph` (action) with snapshot → reverse `ui.dom.restore` (Op); without → non_reversible  
13. Empty idempotency key → refuse (not a global key)  
14. `kv.delete` with `prior` on the Op → reverse `kv.set`; without → non_reversible  
15. Host HMAC on → unsigned/forged Cap refuse; Host HMAC off → legacy Caps still work  
16. Peer never verifies or issues signatures  
17. `Cap.subject` set → `args.subject` must match; blank bind refuses  
18. Ed25519 (`ed25519:`) is Host policy; rotation via `trust_ed25519`; Peer never signs  
19. Unknown / blank `Cap.law_generation` refuses; `accept_generation` opens a dual-speak window  
20. Trace correlates Intents only (`Host::for_trace`); never Cap / undo / resume ticket  

See [HARDENING.md](HARDENING.md), [MATURITY.md](MATURITY.md), and [INVARIANTS.md](INVARIANTS.md).

## Layout

```text
Cargo.toml
HARDENING.md
MATURITY.md
IMPLEMENTATION.md
INVARIANTS.md
GLOSSARY-IMPL.md
.github/workflows/cek.yml
scripts/invariants.sh
scripts/coverage.sh
crates/cek-contract/       # types, actions, Baseline, ui, vectors
crates/cek-host-kernel/    # verify, BoundAsk, project, stores
crates/cek-peer-kernel/    # apply only — no mint
crates/cek-ops-baseline/
crates/cek-ops-ui/         # Peer driver (UI world)
crates/cek-cli/
crates/cek-peer-wasm/      # JSON/WASM apply surface; no mint
ports/cek-peer-ts/
ports/cek-peer-wasm/
```

## Edge cases & testing

- [EDGE_CASES.md](EDGE_CASES.md) — closed and deferred edges
- [TESTING.md](TESTING.md) — unit, vectors, property-style, coverage
- [INVARIANTS.md](INVARIANTS.md) — executable never-regress map
- [HARDENING.md](HARDENING.md) — fail-closed rules
- [GLOSSARY-IMPL.md](GLOSSARY-IMPL.md) — action vs Op, snapshot, scope
