# Python Host runtime

In-process **Host kernel** (verify, project, once, lineage, HMAC + Ed25519). Not law.

```bash
python3 ports/cek-host-py/run_vectors.py crates/cek-contract/vectors
```

Ed25519 is RFC 8032 (stdlib only). Peer-only fixtures are skipped.
There is no apply here — use a Peer runtime for that.
