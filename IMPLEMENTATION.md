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
| `cek-host-kernel` | mint, verify, sealed-args, once, idempotency, BoundAsk, project, lineage, receipts, reverse |
| `cek-peer-kernel` | profile, apply, receipt — **no mint** |
| `cek-ops-baseline` | In-memory kv |
| `cek-cli` | Demo + vector runner |

## Pipeline (Host)

```text
Intent+Cap
  → action match
  → expiry
  → sealed-args bind (if present)
  → once consume
  → BoundAsk
  → project Ops (kv.write → kv.set, …)
  → idempotency record
  → lineage commit (if activity_id)
  → ResultMsg { kind, ops, digest }
```

Peer: `apply(Result)` → `Receipt` → optional `Host::report_receipt`.  
Reverse: `Host::end_activity` → Ops for Peer apply.

## Guarantees

1. Cap refuse → zero Ops; world unchanged  
2. Once Cap second use → refuse  
3. Sealed-args tamper → refuse  
4. Digests start with `cek1:`  
5. Activity end → reverse Ops; landed preferred when receipt reported  
6. Peer has no mint API  

See [HARDENING.md](HARDENING.md) and [MATURITY.md](MATURITY.md).

## Layout

```text
Cargo.toml
HARDENING.md
MATURITY.md
IMPLEMENTATION.md
.github/workflows/cek.yml
crates/cek-contract/
crates/cek-host-kernel/
crates/cek-peer-kernel/
crates/cek-ops-baseline/
crates/cek-cli/
```
