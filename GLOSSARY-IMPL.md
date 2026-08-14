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

Actions are never applied. Ops are never submitted.
