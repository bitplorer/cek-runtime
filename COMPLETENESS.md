# Completeness (implementation framework)

**Complete** = design stated well enough to implement without inventing ambient authority.

## Covered

| Area | Where |
|------|--------|
| Repo glance + problem framing | README |
| Concept explainers | CONCEPTS.md |
| Runtime vs kernel; wire | TOPOLOGY.md |
| Scope vs law | SCOPE.md |
| Contract, vectors, Baseline, manifest | 00-contract/ |
| Host/Peer APIs | 01-kernels/ |
| Submit pipeline | 02-host-pipeline/ |
| Peer apply + receipt | 03-peer-apply/ |
| Cap state machine | 04-cap-machine/ |
| Lineage + reverse | 05-lineage-reverse/ |
| Profiles | 06-profiles/ |
| Isolation | 07-isolation/ |
| Crate layout | 08-layout/ |
| CI + ambient bans | 09-ci/ |
| Ports | 10-ports/ |
| Diagrams | diagrams/ |
| Rationale / contributing | CHOICES.md, CONTRIBUTING.md |

## Not complete (next engineering)

- Published JSON Schema files  
- Executable vector suite in CI  
- Rust Host/Peer source  
- Concrete crypto and once-store backends  

## Verdict

Implementation **design** is complete for a reference Rust dual-kernel runtime.  
Shipping code is next, gated by vectors.
