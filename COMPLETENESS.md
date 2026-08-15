# Completeness (implementation framework)

**Read [README.md](README.md) first** — then [GUIDE.md](GUIDE.md) · [INVARIANTS.md](INVARIANTS.md).

**Complete** = design stated well enough to implement without inventing ambient authority.

## Covered (and shipped)

| Area | Where |
|------|--------|
| Repo glance + problem framing | README |
| Concept explainers | CONCEPTS.md |
| Runtime vs kernel; wire | TOPOLOGY.md |
| Scope vs law | SCOPE.md |
| Contract, vectors, Baseline, manifest | 00-contract/ + `crates/cek-contract` (57 JSON vectors) |
| Host/Peer APIs | 01-kernels/ + `crates/cek-host-kernel` + `cek-peer-kernel` |
| Submit pipeline | 02-host-pipeline/ + `host.rs` |
| Peer apply + receipt | 03-peer-apply/ |
| Cap state machine | 04-cap-machine/ |
| Lineage + reverse | 05-lineage-reverse/ |
| Profiles | 06-profiles/ |
| Isolation | 07-isolation/ |
| Crate layout | 08-layout/ · actual names in IMPLEMENTATION.md |
| CI + ambient bans | 09-ci/ + `scripts/invariants.sh` |
| Ports | 10-ports/ · [PORTS.md](PORTS.md) · `ports/` |
| Diagrams | diagrams/ |
| Rationale / contributing | CHOICES.md, CONTRIBUTING.md |

## Honest residuals (not “shipping code is next”)

v0.1 **code is shipped**: Host + Peer + drivers, 57 vectors, 24 executable invariants, Python Host + JS/TS/WASM Peers, batteries. See README status table.

Still not a published JSON Schema file dump (`cek-contract` types live in Rust). File-backed stores exist in the Rust kernel; Redis does not. Published crates.io kernels are Phase 3.

## Verdict

Implementation **design and reference code** are complete for a dual-kernel runtime. The published Python Host is **`pip install cek-host`** ([cek-python](https://github.com/bitplorer/cek-python)) — `ports/cek-host-py` is a pointer, not a second Host.
