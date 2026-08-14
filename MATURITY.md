# Maturity model — CEK runtime reference

## Status: Stage B (interop-hardening) toward Stage C

| Stage | Meaning | This tree |
|-------|---------|-----------|
| A Core frozen | BoundAsk, refuse, once, Baseline, vectors | **Met** |
| B Interop mature | Digests, sealed-args, receipts, CI, property tables | **Met in reference** |
| C Domain-generic | Domain Op packs without law change | Baseline only; store traits ready for packs |
| D Institutional | Multi-port, dual-speak windows | Not yet |

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
- [x] Executable vectors (31)  
- [x] Unknown-meta ignored on the wire  
- [x] Peer no-mint (CI + `invariants.sh`)  
- [x] CLI demo + vector runner  
- [x] HARDENING.md / INVARIANTS.md  
- [x] GitHub Actions workflow  
- [x] Durable store traits + file backends  
- [x] Fail-closed store-down + concurrent once  
- [x] Property tables (no external proptest)  
- [ ] Second Peer language port  
- [ ] Cap cryptographic signatures  
- [ ] Domain `ui.*` with snapshots  
- [ ] Scope attenuation  

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
