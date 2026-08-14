# Python Host runtime

In-process **Host kernel** (verify, project, once, lineage). Not law.

```bash
python3 ports/cek-host-py/run_vectors.py crates/cek-contract/vectors
```

Ed25519 vectors are skipped (HMAC uses stdlib). Peer-only fixtures are skipped.
There is no apply here — use a Peer runtime for that.
