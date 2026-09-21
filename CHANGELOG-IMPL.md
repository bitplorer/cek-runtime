# Implementation changelog

## 2026-09-21 — PyO3 Cap Door B (owned extract)

- `cek-peer-pyo3` `apply_ops`: Python `list[dict]` / apply-request mapping → owned
  `Op` / `ApplyRequest` in the pyo3 hop → `apply_request` / `apply_world`.
- Door A `apply` / `apply_json` stays open (JSON text). Same Cap algebra. No live
  `PyDict` in `cek-peer-kernel`. GIL released across kernel apply.
- Not a second Peer. Not ux-valio. No workspace version bump.

## 2026-09-20 — Host runtime door (`cek-host-rust`)

- `cek-host-rust` is the native host runtime: own JSON mint/submit wire on `cek-host-kernel`.
- `cek host-json` hops onto that crate. Sessionful wrap; not a second decide engine; not a Python Host twin.
- Public wire verbs stay `mint` | `submit`. Receipt/reverse stay kernel methods, off this door.

## 2026-09-20 — 0.1.3 crates.io republish

- `0.1.3` — republish so peer hops see kernel `apply_world` on crates.io (registry skew fix).

## 2026-09-20 — Internal names match CEK verbs (#13)

- `with_ui_and_policy` → `with_ui_policy` (pairs with `with_policy`).
- PyO3 private helper `kernel_from_req` → `apply_world_from_req` (it wraps `apply_world`).
- PyO3 parity local `via_cli_path` → `via_rust_apply_json` (names the rust hop, not CLI).
- Public JSON field names and `PeerAbi` unchanged.

## 2026-09-20 — UI policy + fail-closed one-shot (#12)

- UI profile honors wire `unknown_op_policy` (`with_ui_policy` / `apply_world`); it no longer drops FailBatch by building `Peer::with_ui()` (Skip).
- One-shot apply no longer maps `Peer::apply` `None` to an empty success receipt (fail closed).
- Vector `unknown_op_ui_fail_batch` plus hop parity tests lock this. CLI / PyO3 share `apply_world`.

## 2026-09-20 — Peer hop graph restore

- `cek-peer-kernel` owns `Peer::apply` plus typed `apply_world` (profile/policy → apply → snapshots).
- `cek-peer-wasm` is `hop: WASM C ABI + own JSON → peer kernel`. The #10 wasm→rust edge is gone (wrong-owner).
- `cek-peer-rust` stays the native peer runtime with its own thin JSON wire. PyO3 and `cek apply` stay on rust only.
- Same JSON field names. No `cek-peer-json` crate. No twin engine.

## 2026-09-05 — Once two-phase hold (LAW §12)

- Residual closed by verification + docs: two-phase once (`ensure_available` before dispatch, `commit` only after successful dispatch, no burn on miss) already held at `5986f30`.
- Host pipeline / Cap machine / hardening / crate docs no longer describe a single `consume_once` step.

## 2026-09-05 — Trace store (LAW §10)

- Persist optional `Intent.trace` on `LineageEntry`; `Host::for_trace` / `LineageBackend::for_trace` groups related Intents.
- Trace remains correlation only — never Cap, undo, or a resume ticket.

## 2026-08-14 — Python Ed25519 + complete DOM driver

- Python Host signs/verifies `ed25519:` (RFC 8032, no extra deps). All 3 Ed25519 vectors pass.
- `DomTree` is a full Peer driver: `#id`, `/path`, insert/remove, text, attrs, HTML render.

Law unchanged. Peer still has no mint.
