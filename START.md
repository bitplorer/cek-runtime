# START — cargo demo is the happy path

**Read this first.** Law is [cek-framework](https://github.com/bitplorer/cek-framework). Python product is [cek-python](https://github.com/bitplorer/cek-python).

```bash
cargo test --workspace
cargo run -p cek-cli -- demo
cargo run -p cek-cli -- vectors crates/cek-contract/vectors
```

`cek-cli demo` is eight scenes: refuse, sealed-args, kv, reverse, ui snapshot, hmac, Ed25519.

Python Host (published, not `ports/cek-host-py`):

```bash
pip install cek-host
python -m cek_host create-app ./hello && python ./hello/app.py
```

Layer 1: [INVARIANTS.md](INVARIANTS.md). Encyclopedia: GUIDE / IMPLEMENTATION / CONCEPTS — each assumes you read this page.
