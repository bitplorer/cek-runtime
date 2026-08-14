# Completeness (implementation framework)

**Complete** = design stated in enough detail to implement without inventing ambient authority.

## Covered

| Area | Where |
|------|--------|
| Scope vs law | SCOPE.md |
| Contract (schemas, vectors, Baseline Ops, manifest) | 00-contract/ |
| Two kernels only; API surface | 01-kernels/ |
| Ordered Host pipeline | 02-host-pipeline/ |
| Peer apply + profile + receipt | 03-peer-apply/ |
| Cap lifecycle machine | 04-cap-machine/ |
| Lineage + reverse plans | 05-lineage-reverse/ |
| Baseline vs production profiles | 06-profiles/ |
| Isolation options | 07-isolation/ |
| Workspace layout | 08-layout/ |
| CI vector gate + ambient bans | 09-ci/ |
| Future ports | 10-ports/ |
| Rationale | CHOICES.md |

## Not complete (expected next engineering)

- Published JSON Schema files (stubs described; fill from CORE shapes)  
- Executable vector suite checked into CI  
- Actual Rust source for Host/Peer  
- Concrete crypto (e.g. Ed25519) and once-store backend  

## Verdict

Implementation **design** is complete for a reference Rust dual-kernel runtime.  
Shipping code is the next phase, gated by vectors.
