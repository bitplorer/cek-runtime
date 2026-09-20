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
| Contract, vectors, Baseline, manifest | 00-contract/ + `crates/cek-contract` (72 JSON vectors — [TESTING.md](TESTING.md)) |
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

v0.1 **code is shipped**: Host + Peer + drivers, **72** vector JSON files, **30** rows in [INVARIANTS.md](INVARIANTS.md) (numbers 1–28 including 10a and 10b), Python Host + JS/TS/WASM Peers, batteries. Maturity checklist: [MATURITY.md](MATURITY.md).

Still not a published JSON Schema file dump (`cek-contract` types live in Rust). File-backed stores exist in the Rust kernel; Redis does not. Kernels are already on crates.io at workspace version **0.1.2**. What’s left to publish is the new hops `cek-peer-rust` and `cek-peer-pyo3` (Trusted Publishing / `publish-crates`).

## Verdict

Implementation **design and reference code** are complete for a dual-kernel runtime. The published Python Host is **`pip install cek-host`** ([cek-python](https://github.com/bitplorer/cek-python)) — `ports/cek-host-py` is a pointer, not a second Host.
