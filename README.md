# CEK Runtime

**Implement a CEK Host (authority) and Peer (apply surface) without ambient power.**

**Start here:** [START.md](START.md)

| | |
|--|--|
| **This repo** | Design playbook **+** reference Rust workspace (`crates/`) |
| **Law** | [cek-framework](https://github.com/bitplorer/cek-framework) |
| **Python Host** | `pip install cek-host` — [cek-python](https://github.com/bitplorer/cek-python) |

| Status | Detail |
|--------|--------|
| **v0.1 code** | Host + Peer + drivers, 57 vectors, 147 tests, Python Host + JS Peer, batteries |
| Proven | Cap refuse → zero Ops; snapshot reverse; scopes cannot widen; Peer no mint |
| Doc | **[START.md](START.md)** · [GUIDE.md](GUIDE.md) · [INVARIANTS.md](INVARIANTS.md) · [PORTS.md](PORTS.md) |

```text
verify → once → project → lineage → Result
```

```bash
cargo test --workspace
cargo run -p cek-cli -- demo
cargo run -p cek-cli -- vectors crates/cek-contract/vectors
```

Contract is the sole interop product. Host is a Cap state machine. Peer is pure apply. `ports/cek-host-py` is **not** a second published Host.
