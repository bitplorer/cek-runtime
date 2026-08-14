# 09 — CI and ambient bans

## Merge gate

Red CORE 19 vector family → block merge on Host or Peer.

## Repo bans

1. No feature flag that skips Cap verify for “trusted” paths  
2. No demo that mutates shared world without Cap  
3. Bootstrap only behind explicit Host-only entry (cfg / module)  
4. Peer package: fail CI if `mint` / `mint_root` symbols appear  
5. Frozen CEK nouns only in public kernel APIs (A10 hygiene)  
6. One Host `submit` orchestrator; one Peer `apply` entry  

## Separate key purposes in tests

Vectors should not use the same key material for Cap and transport.
