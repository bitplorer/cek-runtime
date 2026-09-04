# Glossary — runtime terms (law stays in cek-framework)

| Say this | Meaning in this tree | Not this |
|----------|----------------------|----------|
| **Action** | Intent / Cap verb: `kv.write`, `ui.morph` | An Op name |
| **Op** | Peer-applied data: `kv.set`, `ui.dom.morph` | An Intent |
| **Cap** | Sole authority object | A receipt, a trace, a scope list |
| **Scope** | Optional allow-list on a Cap (`kv:greeting`) | Permission by itself |
| **Attenuate** | Derive a **narrower** Cap (Host policy) | Widen or mint on the Peer |
| **BoundAsk** | Post-verify token; Host-private | A Cap |
| **Result** | Host answer: kind + Ops + digest | World mutation |
| **Receipt** | Peer report of landed / failed | Authority |
| **Snapshot** | Prior value on the Op (`ui.dom.morph.snapshot`, `kv.delete.prior`) | A Host secret |
| **Restore** | Inverse Op `ui.dom.restore` | Compensation Intent |
| **Baseline** | Classic Ops every Peer can apply | The UI pack |
| **Lower** | Project domain Ops to Baseline (`kv.set ui:{target}`) | Change the Cap |
| **Trace** | Correlation id | Permission |
| **subject** | Cap bind of who may present; Intent shows `args.subject` | A trace id |
| **sig** | Host-policy HMAC (`cek1:`) or Ed25519 (`ed25519:`) | Peer authority |
| **dual-speak** | Host accepts current + previous `law_generation` | A second law; Peer mint |
| **driver** | Peer-outer world (`cek-ops-ui`, `cek-ops-baseline`) — [DRIVERS.md](DRIVERS.md) | A third kernel |
| **Context** | Mediated visible world of an Activity (LAW §8) | Ambient authority; a Cap substitute |
| **inject** | Declare what an Activity requires; undeclared access fails closed | A grant of parent rights |
| **limit** | Restrict what an Activity Context may see/do (narrow only) | **isolate**; `Cap.scopes` / `Host::attenuate` |
| **isolate** | Separate a Context slice so names/services do not leak | **limit**; crate `07-isolation/` (process/WASM Peer) |

Actions are never applied. Ops are never submitted.
