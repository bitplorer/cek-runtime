# What lives where

Law is [cek-framework](https://github.com/bitplorer/cek-framework). This repo is the runtime.

| Layer | Path | May contain | Must not contain |
|-------|------|-------------|------------------|
| **Law** | other repo | Meanings, invariants | Code |
| **Contract** | `crates/cek-contract` | Wire types, Baseline catalog, **named** domain Op shapes (for vectors), digests | Host project, Peer apply, mint |
| **Host kernel** | `crates/cek-host-kernel` | Verify, mint, once, BoundAsk, **Baseline** project (`kv.*` / `log.*`), lineage | Domain project (`ui.morph`) |
| **Peer kernel** | `crates/cek-peer-kernel` | Apply **Baseline** Ops, receipts | Mint; UI apply logic |
| **Baseline world** | `crates/cek-ops-baseline` | In-memory kv | Domain worlds |
| **Extensions** | `extensions/` | Host packs + domain worlds | BoundAsk, refuse path, law edits |
| **Ports** | `ports/` | Apply-only Peers (TS, WASM runner) | Mint |

## Two UI crates (both extensions)

| Crate | Role |
|-------|------|
| `extensions/cek-ext-ui` | **Host pack** — Action `ui.morph` → Op `ui.dom.morph` |
| `extensions/cek-ops-ui` | **Peer world** — apply `ui.dom.*` onto a JSON map |

Contract may *name* `ui.dom.morph` so vectors can describe it. The Host kernel does not project it. The Peer kernel does not implement morph/restore; it calls `cek_ops_ui::apply_op` only when constructed with `Peer::with_ui()`.

## Host policy (stays in Host kernel)

HMAC, Ed25519, subject bind, dual-speak, scopes — these are **verify**, not domain packs.

## Ports vs crates

`crates/cek-peer-wasm` is a Rust crate so Cargo can build `wasm32`. The Node runner lives in `ports/cek-peer-wasm`. Same apply-only rule.
