# Concepts at a glance (implementation)

Concise pictures of every major **runtime** idea.  
Law meanings: [cek-framework CONCEPTS](https://github.com/bitplorer/cek-framework/blob/main/CONCEPTS.md).  
Topology (runtime ⊃ kernel): [TOPOLOGY.md](TOPOLOGY.md).

**How to read each block:** what it is → where it sits → what it is *not*.

---

## Big picture

```text
cek-contract  = shared exam (schemas + vectors + Baseline Ops)
Host runtime  ⊃ Host kernel   (decide)
Peer runtime  ⊃ Peer kernel   (apply)
Wire / in-proc = contract messages only — no third kernel in the middle
```

---

## Runtime vs kernel

```text
┌──────────── Host runtime ────────────┐
│ transport · stores · keys · clock    │
│  ┌──────── Host kernel ────────────┐ │
│  │ mint verify once dispatch       │ │
│  │ lineage project reverse Result  │ │
│  └─────────────────────────────────┘ │
└──────────────────────────────────────┘

┌──────────── Peer runtime ────────────┐
│ transport · drivers (DOM, kv, …)     │
│  ┌──────── Peer kernel ────────────┐ │
│  │ profile apply receipt           │ │
│  │ NO mint · NO Cap authority      │ │
│  └─────────────────────────────────┘ │
└──────────────────────────────────────┘
```

| Name | Is | Is not |
|------|----|--------|
| **Host kernel** | L1 decide logic | The whole OS process |
| **Host runtime** | Kernel + network + DB + keys | A third global “cek service” |
| **Peer kernel** | L1 apply logic | Allowed to mint Caps |
| **Peer runtime** | Kernel + drivers + transport | Cap authority |

→ [TOPOLOGY.md](TOPOLOGY.md) · [01-kernels](01-kernels/README.md)

---

## cek-contract

**The only interop product.** Host and Peer are ports that must pass it.

```text
cek-contract/
  schemas/       Intent Cap Result Op lineage receipt profile manifest
  vectors/       CORE 19 families as executable cases
  baseline-ops/  classic data-only Ops + lowering rules
  law-version    which law generation this pack claims
```

| Is | Is not |
|----|--------|
| Shared shapes + tests | The Host or Peer implementation |
| Language-neutral | Optional prose |

If it is not in the contract, it is not CEK interop.

→ [00-contract](00-contract/README.md)

---

## Manifest

**Handshake document** from a Host or Peer process.

```text
manifest {
  law_generation
  profiles[]
  fail_closed { once_store_down, … }
  optional { receipts, idempotency }
}
```

Missing manifest → treat Peer as **Baseline-only**.  
Manifest never grants Cap authority.

---

## BoundAsk

**Token produced only after verify + once/idempotency succeed.**

```text
Intent+Cap → verify → once/idem → BoundAsk → dispatch
                │ refuse              │
                └─ zero Ops           └─ only path to side effects
```

Dispatch must not be callable without a `BoundAsk` (implementation shape).

→ [02-host-pipeline](02-host-pipeline/README.md) · [04-cap-machine](04-cap-machine/README.md)

---

## Host submit pipeline

Ordered, fail closed:

```text
1 verify Cap
2 Context mediate (LAW §8: inject / limit / isolate)
3 once / idempotency
4 reload truth (Host store)
5 dispatch
6 lineage (if required)
7 project Ops to profile ∪ Baseline
8 Result
```

Cap refuse at (1), Context refuse at (2), or store-down at (3) → **no** mutate Ops, **no** cause.

→ [02-host-pipeline](02-host-pipeline/README.md)

---

## Cap machine (code)

Conceptual states enforced in Host:

```text
Minted → Active → Consumed(once) | Expired | Revoked
```

| Transition | Rule |
|------------|------|
| once | Atomic consume **before** effects |
| replay Consumed | Refuse |
| Expired/Revoked | Verify refuses (revoked is Host registry state, not a Cap wire field) |
| revoke | `Host::revoke` → reverse that Cap's lineage (LAW §9) |

→ [04-cap-machine](04-cap-machine/README.md)

---

## Peer apply

```text
Result.ops ──in order──► apply each Op
                │
         known? ──no──► profile policy (ignore / soft / strict)
                │ yes
                ▼
         Landed | Failed ──► optional receipt
```

Peer never rewrites Host authorized set. Crash ≠ Host inventing truth.

→ [03-peer-apply](03-peer-apply/README.md)

---

## Receipt

```text
receipt { landed_ops[], failed_ops[] }   // not a Cap
```

| Receipt | Reverse uses |
|---------|----------------|
| Present | **landed** set |
| Absent | **authorized** set |

Production-v1 profile should require receipts.

→ [05-lineage-reverse](05-lineage-reverse/README.md)

---

## Reverse plan classes

At lineage commit, every mutate cause gets a class:

```text
inverse        → undo Ops
compensation   → Intents under recovery Cap (LAW §13; Host::mint_recovery)
non_reversible → mark + audit; never claim clean reverse
```

DOM morph usually needs snapshot/restore (compensation) or inverse patch data kept at lineage time. **Apply complete ≠ undo.**

→ [05-lineage-reverse](05-lineage-reverse/README.md)

---

## Trace (LAW §10)

**Correlation of related Intents.** Optional on Intent and on the lineage entry.

```text
Intent.trace?  →  LineageEntry.trace?  →  Host::for_trace / LineageBackend::for_trace
```

| Is | Is not |
|----|--------|
| Group / query key | Permission, execute, or undo |
| Optional (no-trace is OK) | A Cap, a session, or a resume ticket |

Each step still needs a Cap. End reverses lineage (not reverse-by-trace). Resume needs a fresh Cap.

→ [05-lineage-reverse](05-lineage-reverse/README.md)

---

## Baseline Ops vs domain Ops

```text
Baseline (permanent)     e.g. kv.set, kv.delete, log.append
Domain (L5, optional)    e.g. ui.dom.morph, ui.dom.restore

Host may lower rich outcomes → Baseline forms when Peer profile is thin
```

Ops stay **data only** — no eval, no closures in Baseline.

→ [00-contract](00-contract/README.md) · [06-profiles](06-profiles/README.md)

---

## Profiles

| Profile | Expectation |
|---------|-------------|
| **Baseline** | Classic Ops; receipts/idempotency optional |
| **production-v1** | Idempotency + receipts + fail-closed once-store |

Profile negotiates **apply ability**, not Cap power.

→ [06-profiles](06-profiles/README.md)

---

## Activity & Context (LAW §8)

**Activity** = bounded work; on end Host reverses lineage.  
**Context** = mediated visible world of that Activity — not ambient authority, not a Cap substitute.

```text
inject  → declare required names/services; undeclared access fails closed
limit   → restrict what may be seen or done (narrow only)
isolate → separate a slice so names/services do not leak across Activities
```

`limit` ≠ `isolate`. Both only **narrow** (A8). Host mediates on submit.

→ [cek-framework CORE/07](https://github.com/bitplorer/cek-framework/blob/main/CORE/07-activity-context.md)

---

## Isolation modes

Same kernel APIs; stronger walls optional:

```text
module split  → separate crates (default v0)
process split → Host process + Peer worker, Ops over IPC
WASM Peer     → Host feeds Ops into component
```

→ [07-isolation](07-isolation/README.md)

---

## Crate layout (reference)

```text
cek-contract
cek-types                 (optional; or inside contract)
cek-host-kernel-rust
cek-peer-kernel-rust
cek-ops-baseline
cek-cli                   vectors + demos
```

Host must not depend on Peer internals. Peer must not link mint.

→ [08-layout](08-layout/README.md)

---

## Vectors & CI gate

```text
PR → run contract vectors → Peer mint symbol check → merge or block
```

Red Cap-refuse vector (world changed) = not CEK-aligned.

→ [09-ci](09-ci/README.md)

---

## Wire delivery patterns

| Pattern | Who moves Result to Peer |
|---------|---------------------------|
| Caller-mediated | L7 app |
| Host pushes | Host runtime |
| Queue/bus | Message bus (not a kernel) |

→ [TOPOLOGY.md](TOPOLOGY.md)

---

## Ports & L7

| Role | Languages in *this* framework |
|------|-------------------------------|
| Host kernel | **Rust only** |
| Peer kernel | **Rust only** (other Peer *ports* later) |
| L7 caller | Any — holds Cap, calls submit |

→ [10-ports](10-ports/README.md)

---

## One-line map

| Concept | One line |
|---------|----------|
| **cek-contract** | Shared schemas + vectors |
| **Host runtime** | Process wrapping Host kernel |
| **Peer runtime** | Process wrapping Peer kernel |
| **BoundAsk** | Post-verify token; required for dispatch |
| **Receipt** | What Peer landed; guides reverse |
| **Manifest** | What this process claims to speak |
| **Baseline Ops** | Forever-interop effect catalog |
| **Vector gate** | Red test blocks merge |
