# Scope — Law vs Implementation

## Belongs in cek-framework (law)

- META method, axioms, vocabulary, layers  
- Host/Peer *roles*, Cap/Intent/Ops *concepts*  
- Lineage, reverse, Baseline, profile *as law*  
- Kill criteria, charter, conformance *families*  
- Encoding-free conceptual shapes  

## Belongs here (implementation)

- Programming languages for kernels  
- Crate/module layout  
- Cap as a typed state machine in code  
- Submit orchestrator API surface  
- Peer isolation technology (process, WASM, in-process modules)  
- CI rules, lint bans, merge gates  
- Concrete schema files and vector JSON  
- Production profile packaging  
- Crypto/store choices (must still satisfy Cap binds)  
- How L7 callers talk to Host (IPC, HTTP, in-process)  

## Hard rule

If a change would rename a frozen kernel concept, alter axioms, or add a third L1 role — it is a **charter amendment** in cek-framework, not a patch here.

If a change only affects Rust APIs, CI, isolation, or vector *encoding* — it lives here.
