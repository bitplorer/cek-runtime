# Implementation diagrams

Mermaid sources for the CEK **runtime** design.  
Law-level diagrams stay in [cek-framework/diagrams](https://github.com/bitplorer/cek-framework/tree/main/diagrams).

Plain-text topology (no Mermaid required): [TOPOLOGY.md](../TOPOLOGY.md).

| File | Content |
|------|---------|
| [01-system-boundary.mmd](01-system-boundary.mmd) | L7 caller, Host, Peer, lineage |
| [02-submit-pipeline.mmd](02-submit-pipeline.mmd) | Host submit state machine |
| [03-cap-machine.mmd](03-cap-machine.mmd) | Cap lifecycle in code |
| [04-peer-apply.mmd](04-peer-apply.mmd) | Ordered apply + receipt |
| [05-reverse.mmd](05-reverse.mmd) | Activity end / reverse plans |
| [06-contract-product.mmd](06-contract-product.mmd) | Contract as sole interop product |
| [07-crate-layout.mmd](07-crate-layout.mmd) | Workspace dependency shape |
| [08-isolation.mmd](08-isolation.mmd) | Module / process / WASM Peer |
| [09-happy-path.mmd](09-happy-path.mmd) | End-to-end sequence |
| [10-edge-defaults.mmd](10-edge-defaults.mmd) | Deterministic edge defaults |
| [11-ci-gate.mmd](11-ci-gate.mmd) | Vector merge gate |
| [12-law-runtime-split.mmd](12-law-runtime-split.mmd) | What lives in which repo |
| [13-runtime-kernel.mmd](13-runtime-kernel.mmd) | Runtime wraps kernel; wire between runtimes |
