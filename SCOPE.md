# Scope — law vs implementation

| In [cek-framework](https://github.com/bitplorer/cek-framework) | In **this** repo |
|---------------------------------------------------------------|------------------|
| META method, axioms, vocabulary, layers | Kernel languages (Rust reference) |
| Host/Peer *roles*; Cap/Intent/Ops *concepts* | Crate and module layout |
| Lineage, reverse, Baseline, profile as law | Cap as a typed state machine in code |
| Kill criteria, charter, conformance *families* | Submit API surface, isolation, CI bans |
| Encoding-free conceptual shapes | Schema files, vector JSON, crypto/store choices |
| | How L7 callers reach Host (IPC, HTTP, in-process) |

## Hard rule

| Change type | Where |
|-------------|--------|
| Rename frozen concept, change axiom, add third L1 role | **Charter amendment** in cek-framework |
| Rust API, CI, isolation, vector encoding | **This** repo |

Do not weaken fail-closed Cap verify or Peer no-mint in the name of convenience.
