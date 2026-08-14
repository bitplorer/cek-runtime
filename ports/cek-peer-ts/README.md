# cek-peer-ts

Apply-only TypeScript Peer. **No mint.** Same Baseline + `ui.dom.*` apply
semantics as `cek-peer-kernel`.

```bash
# from workspace root
node --experimental-strip-types --no-warnings \
  ports/cek-peer-ts/src/run-vectors.ts crates/cek-contract/vectors
```

Self-check plus any vector that ships `peer_result` (Peer-only cases).
Host-projected cases stay on the Rust runner — this port must not decide.

Law: https://github.com/bitplorer/cek-framework
