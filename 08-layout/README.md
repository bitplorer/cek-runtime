# 08 — Crate layout

```text
cek-runtime/                 # this design repo (docs)
  …

# Suggested code workspace (future / adjacent)
cek/
  cek-contract/              # schemas, vectors, law-version
  cek-types/                 # shared Intent/Cap/Result/Op types (or inside contract)
  cek-host-kernel-rust/      # Host kernel
  cek-peer-kernel-rust/      # Peer kernel
  cek-ops-baseline/          # classic Ops apply handlers
  cek-cli/                   # run vectors, S1–S8 demos
```

## Dependency rules

| Crate | May depend on |
|-------|----------------|
| contract | nothing kernel-specific |
| host | contract, types |
| peer | contract, types, ops-baseline |
| host | **must not** depend on peer internals |
| peer | **must not** depend on host mint |

Shared types prevent Host/Peer drift.
