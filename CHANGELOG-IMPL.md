# Implementation changelog

## 2026-09-05 — Trace store (LAW §10)

- Persist optional `Intent.trace` on `LineageEntry`; `Host::for_trace` / `LineageBackend::for_trace` groups related Intents.
- Trace remains correlation only — never Cap, undo, or a resume ticket.

## 2026-08-14 — Python Ed25519 + complete DOM driver

- Python Host signs/verifies `ed25519:` (RFC 8032, no extra deps). All 3 Ed25519 vectors pass.
- `DomTree` is a full Peer driver: `#id`, `/path`, insert/remove, text, attrs, HTML render.

Law unchanged. Peer still has no mint.
