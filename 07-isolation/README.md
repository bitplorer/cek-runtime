# 07 — Isolation

Peer is treated as untrusted for Host truth even when written in Rust.

This crate note is **process/WASM Peer split**, not LAW §8 **isolate** (Context slice). Context `inject` / `limit` / `isolate` live on Host.

## Options (increasing assurance)

| Mode | Description |
|------|-------------|
| Module split | Separate crates; Peer has no mint symbols |
| Process split | Host process + Peer worker; Ops over IPC |
| WASM component | Peer as component; Host feeds Ops |

## What Peer must not access

- Cap signing/verify private material  
- once-store write authority  
- lineage authority DB as Cap substitute  
- bootstrap mint entry  

## Recommendation

Ship module split first with CI symbol bans.  
Add process or WASM isolation when multi-tenant or untrusted Peer code is in scope.
