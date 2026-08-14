# Maturity model — CEK runtime reference

## Status: Stage B (interop-hardening) toward Stage C

| Stage | Meaning | This tree |
|-------|---------|-----------|
| A Core frozen | BoundAsk, refuse, once, Baseline, vectors | **Met** |
| B Interop mature | Digests, sealed-args, receipts, CI | **Met in reference** |
| C Domain-generic | Domain Op packs without law change | Baseline only; path clear |
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

## Completeness checklist

- [x] BoundAsk  
- [x] Once before effects  
- [x] Sealed-args enforcement  
- [x] Result digests (`cek1:`)  
- [x] Idempotency bind store  
- [x] Receipt annotation  
- [x] Landed-first reverse preference  
- [x] Reverse classes  
- [x] Executable vectors  
- [x] Peer no-mint  
- [x] CLI demo + vector runner  
- [x] HARDENING.md  
- [x] GitHub Actions workflow  
- [ ] Second Peer language port  
- [ ] Durable once/lineage backends  
- [ ] Cap cryptographic signatures  
- [ ] Domain `ui.*` with snapshots  

## Consistency glossary (code ↔ law)

| Law term | Code |
|----------|------|
| Cap | `cek_contract::Cap` |
| Intent | `cek_contract::Intent` |
| Ops | `cek_contract::Op` |
| Result | `cek_contract::ResultMsg` |
| Host | `cek_host_kernel::Host` |
| Peer | `cek_peer_kernel::Peer` |
| lineage | `LineageStore` / `LineageEntry` |
| reverse | `Host::end_activity` |
| receipt | `Receipt` + `Host::report_receipt` |
| Baseline | `cek_contract::baseline` |
| BoundAsk | `cek_host_kernel::BoundAsk` |
