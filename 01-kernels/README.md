# 01 — Kernels

L1 has exactly two implementations in this framework.

| Crate | Role |
|-------|------|
| `cek-host-kernel` | Decide |
| `cek-peer-kernel` | Carry out (`Peer::apply` + typed `apply_world` helper) |

**Runtime vs kernel:** the kernel is the CEK core; the runtime is the process that wraps it (transport, stores, drivers). Full picture: [TOPOLOGY.md](../TOPOLOGY.md).

```text
Host runtime ⊃ Host kernel
Peer runtime ⊃ Peer kernel
Wire carries contract messages between runtimes — not a third kernel in the middle.
```

## Host API (minimal)

```text
mint(policy) -> Cap
mint_recovery(...) -> Cap          # LAW §13 Host-only Recovery Cap; never Peer root
inject(activity_id, names) -> Context  # LAW §8 declare required names/services
limit(activity_id, names) -> Context   # LAW §8 restrict (narrow only; ≠ isolate)
isolate(activity_id) -> Context        # LAW §8 slice; names/services do not leak
submit(Intent, Cap) -> Result          # orchestrates the pipeline; must not skip stages
end_activity(activity_id) -> ReverseOutcome
revoke(cap_id) -> ReverseOutcome       # LAW §5 Active→Revoked; LAW §9 reverse
manifest() -> Manifest
```

Host **must not** expose a free world-mutate path outside Ops emission + lineage write.

## Peer API (minimal)

```text
profile() -> Profile
apply(Result) -> Option<Receipt>
```

Typed helper **in the same crate** (not a second engine): `apply_world` (profile/policy → `Peer::apply` → receipt + snapshots).  
`cek-peer-wasm` and `cek-peer-rust` are sibling hops that serde JSON onto that helper. PyO3 and `cek apply` hop onto rust, not wasm.  
`cek-host-rust` is the native host JSON door on `cek-host-kernel`. `cek host-json` hops onto it. See [TOPOLOGY.md](../TOPOLOGY.md).

Peer **must not** expose:

- `mint` / `mint_root`  
- Cap verify as authority source  
- lineage authority or Cap key material  

## In-process vs out-of-process

Both allowed. Role split is in **types and APIs**, not in process count.  
Mint code must not be reachable from the apply path (module visibility + CI).

## Not kernels

Caller · bootstrap config · lineage DB · recovery Cap (still a Cap) · profile declaration · transport · vector runner · message bus
