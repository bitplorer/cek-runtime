# 04 — Cap machine

Implement Cap as an explicit state machine (conceptual states from CORE 08).

```text
Minted → Active → Consumed(once) | Expired | Revoked
```

## Code-facing rules

| Transition | Enforcement |
|------------|-------------|
| verify | Only Active Caps pass; Expired/Revoked/Consumed refuse |
| once consume | **Before** dispatch side-effects; atomic vs concurrent submit |
| replay Consumed | Refuse |
| revoke | `Host::revoke` — Cap is dead afterwards (`verify_cap` / submit refuse); reverse causes under that Cap (LAW §5 Active→Revoked, LAW §9) |

## Binds checked at verify

- integrity  
- action match  
- sealed args match  
- validity window (Host clock policy)  
- optional subject / scopes  

Sealed mismatch → refuse; no side-effects.

Revoked is Host registry state, not a Cap wire field.

## Key material

Cap authority material ≠ transport keys ≠ telemetry keys.  
Separate in types and storage.

## Attenuation

`limit` only narrows. Represent attenuated Caps as derived Active Caps with smaller binds — never wider than parent.
