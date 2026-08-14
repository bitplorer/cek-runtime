# Reference implementation (Rust)

Runnable vertical slice for CEK Host + Peer. Design docs in this repo; law in [cek-framework](https://github.com/bitplorer/cek-framework).

## What shipped (v0.1)

| Crate | Role |
|-------|------|
| `cek-contract` | Intent, Cap, Result, Op, lineage, receipt, profile, manifest; Baseline helpers; vector load/check |
| `cek-host-kernel` | Ordered submit: verify → once → BoundAsk → project → lineage → Result; `end_activity` reverse |
| `cek-peer-kernel` | `apply` only — **no mint API** |
| `cek-ops-baseline` | In-memory kv store |
| `cek-cli` | `cek demo`, `cek vectors` |

## Guarantees proven by tests + vectors

1. Cap action mismatch / expiry / once-replay → `authority_refusal`, **zero** Ops, world unchanged  
2. Valid Cap + `kv.write` → `kv.set` Baseline Op lands on Peer  
3. Activity end → inverse Ops (`kv.set` → `kv.delete` in v0 without snapshot)  
4. Peer has no `mint`  

## Commands

```bash
cargo test --workspace
cargo run -p cek-cli -- demo
cargo run -p cek-cli -- vectors crates/cek-contract/vectors
```

## Aging rules (do not regress)

- Contract fields: additive only; Ops remain data (`ns`, `name`, `payload`).  
- Host must not depend on Peer internals.  
- `BoundAsk` only after Cap verify + once — no Cap-skip path.  
- `authority_refusal` must never carry mutate Ops (enforced in vector check).  
- Vectors gate merge; red vector = not CEK-aligned.  

## Layout

```text
Cargo.toml                 workspace
crates/cek-contract/       types + vectors/*.json + law-version.txt
crates/cek-host-kernel/    Host
crates/cek-peer-kernel/    Peer
crates/cek-ops-baseline/   drivers
crates/cek-cli/            demo + vector runner
```

## Not yet (intentionally)

Crypto Cap signatures · durable once/lineage DB · production receipts wire · domain `ui.dom.*` · WASM isolation · multi-language SDKs  

Those extend this slice without changing the BoundAsk / refuse / Baseline pipeline.
