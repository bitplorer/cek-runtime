# Maturity model — CEK runtime reference

## Status: Stage C (domain-generic) underway

| Stage | Meaning | This tree |
|-------|---------|-----------|
| A Core frozen | BoundAsk, refuse, once, Baseline, vectors | **Met** |
| B Interop mature | Digests, sealed-args, receipts, CI, property tables | **Met** |
| C Domain-generic | Domain Op packs without law change | **`ui.*` pack + snapshot reverse + scopes** |
| D Institutional | Multi-port, dual-speak windows | **TS + WASM Peers; law-generation window** |

## Invariants (must not regress)

1. Cap-only authority at kernel boundary  
2. Ops are data  
3. Host decides; Peer applies  
4. Fail closed on unclear authority  
5. Honest reverse (or non_reversible mark)  
6. Baseline Ops remain valid  
7. trace is not permission  
8. Contract + vectors define interop  

See [INVARIANTS.md](INVARIANTS.md) for the executable map.

## Completeness checklist

- [x] BoundAsk  
- [x] Once before effects  
- [x] Sealed-args enforcement  
- [x] Result digests (`cek1:`) + FIPS SHA-256 fixtures  
- [x] Idempotency bind store  
- [x] Idempotency **before** once-ensure (retry of once-Cap)  
- [x] Receipt annotation  
- [x] Landed-first reverse preference  
- [x] Reverse classes (Inverse / Compensation / NonReversible)  
- [x] Executable vectors (57)  
- [x] Unknown-meta ignored on the wire  
- [x] Peer no-mint (CI + `invariants.sh`, Rust + TS + WASM)  
- [x] Domain `ui.*` with snapshot reverse  
- [x] Scope attenuation (Host policy; no widen)  
- [x] Second Peer language port (TS apply-only)  
- [x] Snapshot reverse for `kv.delete` prior value  
- [x] WASM apply-only Peer  
- [x] Cap HMAC (optional Host policy; RFC 4231)  
- [x] Subject bind (`Cap.subject` / `args.subject`)  
- [x] llvm-cov HTML (`scripts/llvm-cov.sh`, CI artifact)  
- [x] Ed25519 Host policy + rotation window  
- [x] Dual-speak law-generation window  

## Consistency glossary (code ↔ law)

| Law term | Code |
|----------|------|
| Cap | `cek_contract::Cap` |
| Intent | `cek_contract::Intent` |
| Ops | `cek_contract::Op` |
| Result | `cek_contract::ResultMsg` |
| Host | `cek_host_kernel::Host` |
| Peer | `cek_peer_kernel::Peer` |
| lineage | `LineageBackend` / `LineageEntry` |
| reverse | `Host::end_activity` |
| receipt | `Receipt` + `Host::report_receipt` |
| Baseline | `cek_contract::baseline` |
| BoundAsk | `cek_host_kernel::BoundAsk` |
| once store | `OnceBackend` (`OnceStore` / `FileOnceStore`) |
| action | `cek_contract::actions` (`kv.write`, `ui.morph`) |
| ui Op | `ui.dom.morph` / `ui.dom.restore` |
| scope | `Cap.scopes` + `check_scopes` / `Host::attenuate` |
| Cap HMAC | `Cap.sig` + `Host::with_hmac_key` |
| Ed25519 | `Host::with_ed25519` / `trust_ed25519` |
