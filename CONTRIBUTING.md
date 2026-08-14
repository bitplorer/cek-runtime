# Contributing

This repository is the **implementation playbook** for CEK. It must not redefine law.

## Welcome

- Clearer docs and diagrams (keep plain-text readable without Mermaid)  
- Stronger edge-case notes aligned with [cek-framework](https://github.com/bitplorer/cek-framework) CORE 24  
- Contract/vector drafts that test existing law  
- Future: Rust Host/Peer code that passes vectors  

## Do not

- Rename Cap, Intent, Ops, Host, Peer, lineage, Baseline  
- Add a third L1 kernel role  
- Add Cap-skip / trusted-mode flags  
- Put mint on the Peer  
- Treat docs here as overriding [cek-framework](https://github.com/bitplorer/cek-framework)  

Law changes go through the framework [CHARTER](https://github.com/bitplorer/cek-framework/blob/main/CHARTER.md).

## Definition of done for code (when added)

See README — vectors green, Cap refuse is effect-free, Peer has no mint-root API.
