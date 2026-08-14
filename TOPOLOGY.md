# Topology — runtime vs kernel

**Kernels sit inside runtimes.** They are not a third hop on the wire.

```text
Host runtime  = process that contains the Host kernel (+ store, transport)
Peer runtime  = process that contains the Peer kernel (+ drivers, transport)
```

There is no required central “cek-runtime service.” Host runtime and Peer runtime *are* the deployed CEK runtime.

---

## Layering (mobile-safe)

**Host runtime**

| Layer | Contents |
|-------|----------|
| Outer | transport · once-store · lineage DB · Cap keys · clock |
| Inner **Host kernel** | mint · verify · once · dispatch · lineage · project · reverse · Result |

**Peer runtime**

| Layer | Contents |
|-------|----------|
| Outer | transport · apply drivers (DOM, kv, device, …) |
| Inner **Peer kernel** | profile · apply Ops · optional receipt · **no mint** |

Between them: **wire** carries `Intent+Cap` / `Result{Ops}` / `receipt`.

| Name | What it is |
|------|------------|
| **Host kernel** | CEK decide logic (L1 role) |
| **Host runtime** | Kernel + process: network, DB, keys, config |
| **Peer kernel** | CEK apply logic (L1 role) |
| **Peer runtime** | Kernel + process: network + drivers |

Wide ASCII form (desktop): [diagrams/13-runtime-kernel.mmd](diagrams/13-runtime-kernel.mmd)

---

## Where they sit in the flow

```text
L7 app
  → Host runtime → Host kernel     (authority)
  → Result{Ops}
  → Peer runtime → Peer kernel     (apply)
  → optional receipt → Host kernel (lineage / later reverse)
```

| Connection | How |
|------------|-----|
| **On the wire** | Host *runtime* ↔ Peer *runtime* (or app in the middle), **contract** messages |
| **In-process** | Runtime calls kernel APIs (`submit`, `apply`) as functions |

---

## What does not sit in the middle

```text
App → “cek-runtime service” → Host kernel → Peer kernel   ← not required
```

No broker kernel. A message bus only **moves** messages; it is not Cap authority.

---

## Wire payloads (contract)

| Direction | Payload |
|-----------|---------|
| Caller → Host | Intent + Cap |
| Host → Caller / Peer | Result (`ops[]` or refusal/error) |
| Peer → Host | optional receipt |
| Host → Peer (reverse) | Result with inverse/restore Ops |

Handshake: **manifest** (`law_generation`, profiles, fail-closed). Missing → Baseline-only Peer.

---

## Three delivery patterns

1. **Caller-mediated** — app calls Host, then Peer with the same `Result`  
2. **Host pushes** — Host calls Peer apply after submit  
3. **Queue/bus** — contract messages on a topic (bus is not a kernel)  

---

## One line

**Host kernel lives inside the Host runtime; Peer kernel lives inside the Peer runtime. They connect by contract messages between those runtimes (or by in-process calls).**
