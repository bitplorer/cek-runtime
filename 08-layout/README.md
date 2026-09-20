# 08 — Crate layout

This repo **is** the code workspace. Names below are the crates on disk — not `cek-*-kernel-rust`, and not a future adjacent tree.

```text
cek-runtime/
  crates/cek-contract/         # types, vectors, Baseline (no cek-types crate)
  crates/cek-host-kernel/      # Host kernel
  crates/cek-host-rust/        # hop: native JSON → host kernel
  crates/cek-peer-kernel/      # Peer kernel: Peer::apply + apply_world
  crates/cek-ops-baseline/     # Peer driver: kv
  crates/cek-ops-ui/           # Peer driver: ui / DOM
  crates/cek-peer-wasm/        # hop: WASM C ABI + own JSON → kernel
  crates/cek-peer-rust/        # hop: native JSON → kernel
  crates/cek-peer-pyo3/        # hop → cek-peer-rust (not wasm)
  crates/cek-cli/              # vectors, demo; cek apply → peer-rust; cek host-json → host-rust
```

## Host / Peer hops

```text
cek-host-kernel          ← decide engine
└── cek-host-rust        ← kernel only; native JSON door
      └── cek host-json (cli)
```

## Peer hops (#11)

One apply engine. wasm and rust are **siblings** on the kernel. PyO3 and the CLI hop onto rust. There is no wasm→rust edge (that was #10, wrong-owner, gone).

```text
cek-peer-kernel          ← Peer::apply + apply_world
├── cek-peer-wasm        ← kernel only; own JSON + C ABI
└── cek-peer-rust        ← kernel only; native JSON door
      ├── cek-peer-pyo3
      └── cek apply (cli)
```

Full picture: [TOPOLOGY.md](../TOPOLOGY.md).

## Dependency rules

| Crate | May depend on |
|-------|----------------|
| contract | nothing kernel-specific |
| host | contract |
| peer kernel | contract, ops-baseline, ops-ui |
| peer wasm / rust | peer kernel (not each other) |
| pyo3 / `cek apply` | `cek-peer-rust` |
| host rust / `cek host-json` | host kernel (not peer crates) |
| host | **must not** depend on peer internals |
| peer | **must not** depend on host mint |

Shared types live in `cek-contract` so Host/Peer do not drift.
