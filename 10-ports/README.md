# 10 — Ports

Additional Host or Peer **implementations** are allowed by law.  
This framework’s *reference* kernels stay Rust.

## When to add a Peer port

| Surface | Example port |
|---------|----------------|
| Browser DOM | **Shipped:** TypeScript apply-only (`ports/cek-peer-ts`), JS runtime (`ports/cek-peer-js`), WASM hop (`crates/cek-peer-wasm` + `ports/cek-peer-wasm`) |
| Agent / server already in Python | **Shipped in-process ABI:** `cek-peer-pyo3` hops onto `cek-peer-rust` (not wasm, not a second kernel). A taught Python carrier remains a follow-up in cek-python |
| MCU / device | C/Rust embedded Peer with tiny profile |

Each port:

1. Declares a profile  
2. Passes applicable vector families  
3. Never mints root Caps  

Hops are not extra engines. wasm and rust serde onto `cek-peer-kernel::apply_world`. PyO3 and `cek apply` hop onto rust. [TOPOLOGY.md](../TOPOLOGY.md).

## Host ports

A second Host language is **out of scope** for this framework’s reference path (user choice: Rust only).  
If added later: same vectors, same Cap binds, explicit cross-Host trust policy.  
Published Python Host is `pip install cek-host` ([cek-python](https://github.com/bitplorer/cek-python)); `ports/cek-host-py` is a contract-vector sketch, not a second kernel.

## L7 callers

Any language may hold a Cap and call Host `submit` over IPC/HTTP.  
That does not make them kernels.
