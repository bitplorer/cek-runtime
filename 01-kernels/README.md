# 01 — Kernels

L1 has exactly two implementations in this framework.

| Crate | Role |
|-------|------|
| `cek-host-kernel-rust` | Decide |
| `cek-peer-kernel-rust` | Carry out |

## Host API (minimal)

```text
mint(policy) -> Cap
submit(Intent, Cap) -> Result          # orchestrates the pipeline; must not skip stages
end_activity(activity_id) -> ReverseOutcome
revoke_cap(cap_id) -> ReverseOutcome   # policy-scoped
manifest() -> Manifest
```

Host **must not** expose a free world-mutate path outside Ops emission + lineage write.

## Peer API (minimal)

```text
profile() -> Profile
apply(Result) -> Option<Receipt>
```

Peer **must not** expose:

- `mint` / `mint_root`  
- Cap verify as authority source  
- lineage authority or Cap key material  

## In-process vs out-of-process

Both allowed. Role split is in **types and APIs**, not in process count.  
Mint code must not be reachable from the apply path (module visibility + CI).

## Not kernels

Caller · bootstrap config · lineage DB · recovery Cap (still a Cap) · profile declaration · transport · vector runner
