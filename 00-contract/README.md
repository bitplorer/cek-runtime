# 00 — Contract

The **contract** is the only interop product. Kernels are ports that pass it.

## Rule

If it is not in the contract (schemas + vectors + Baseline Ops + law generation), it is not CEK interop.

## Artifacts

```text
cek-contract/
  schemas/           # Intent Cap Result Op lineage receipt profile manifest
  vectors/           # CORE 19 families as JSON
  baseline-ops/      # classic Op catalog + lowering rules
  law-version.txt    # law generation id
```

## Conceptual types (encoding free; implement as JSON Schema first)

| Type | Required conceptual fields |
|------|---------------------------|
| **Intent** | action, args (sealed + open), Cap, optional trace |
| **Cap** | action bind, sealed-args bind, validity; optional subject, scopes, once |
| **Result** | ok \| authority_refusal \| dispatch_error; ops[]; error? |
| **Op** | ns, name, payload (data only) |
| **lineage entry** | cap, activity?, trace?, action, sealed_ref, authorized_ops, reverse_plan |
| **receipt** | landed_ops, failed_ops (not a Cap) |
| **profile** | apply_set, unknown_op_policy |
| **manifest** | law_generation, profiles[], fail_closed, optional families |

## Vector families (must be executable)

From CORE 19 — each as versioned JSON cases:

| Family | Must show |
|--------|-----------|
| Cap verify | success; bad integrity; action mismatch; sealed mismatch; expired |
| Single-use | consume-before-effects; second use fails; store down refuses |
| Baseline apply | minimal profile applies classic Ops |
| Baseline lowering | rich outcome → classic Ops |
| Unknown meta | ignored on Baseline |
| Unknown Ops | no kernel crash |
| Lineage | recorded on revocable/endable path |
| Reverse on end | not silent success on failure |
| Trace | groups; grants no authority |
| Attenuation | limited Cap cannot widen |
| Peer limits | Peer cannot mint root |

## Manifest handshake

Missing manifest → assume Baseline-only Peer.  
Manifest never grants Cap authority.

## Deterministic projection

Same Intent outcome + same profile + same projection rules → same Ops bytes.
