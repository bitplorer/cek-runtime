# Python Host runtime

In-process **Host kernel**: verify, project, once, lineage, HMAC + Ed25519.  
Not law. **Does not apply Ops** — hand the Result to a Peer.

Completeness matrix: [PORTS.md](../../PORTS.md).

## Has

- Cap verify (action, expiry, sealed-args, scopes, subject, law generation)
- Project: `kv.write` / `kv.delete` / `log.append` / `ui.morph` / `ui.restore`
- Once after successful project; idempotency before once
- HMAC `cek1:` and Ed25519 `ed25519:` (RFC 8032, stdlib)
- Lineage reverse; landed-first when `report_receipt`
- Result `digest`

## Does not have

- File-backed stores, `attenuate`, extra Ed25519 trust keys, Baseline `lower_ops`

## Run

```bash
python3 ports/cek-host-py/run_vectors.py crates/cek-contract/vectors
python3 ports/cek-host-py/test_batteries.py
echo '{"action":"kv.write","args":{"key":"a","value":1},"cap":{"id":"c","action":"kv.write","once":false}}' \
  | python3 ports/cek-host-py/runtime.py --now 1000
```

51 CORE vectors pass. Peer-only fixtures are skipped (no apply here).
