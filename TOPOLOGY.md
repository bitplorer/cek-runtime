# Topology — runtime vs kernel

**Kernels sit inside runtimes.** They are not a third hop on the wire.

```text
Host runtime  = process that contains the Host kernel (+ store, transport)
Peer runtime  = process that contains the Peer kernel (+ drivers, transport)
```

There is no required central “cek-runtime service.” Host runtime and Peer runtime *are* the deployed CEK runtime.

---

## Layering

```text
┌──────────────────────────── Host runtime ────────────────────────────┐
│  transport (HTTP/IPC/…)                                              │
│  once-store · lineage DB · Cap keys · clock                          │
│  ┌────────────────────── Host kernel ─────────────────────────────┐  │
│  │  mint · verify · once/idempotency · dispatch · lineage ·        │  │
│  │  project · reverse · Result                                     │  │
│  └────────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────────┘
                                    │
                         wire: Intent+Cap / Result{Ops} / receipt
                                    │
┌──────────────────────────── Peer runtime ────────────────────────────┐
│  transport                                                           │
│  apply drivers (DOM, kv, device, …)                                  │
│  ┌────────────────────── Peer kernel ─────────────────────────────┐  │
│  │  profile · apply Ops in order · optional receipt                │  │
│  │  NO mint · NO Cap authority                                     │  │
│  └────────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────────┘
```

| Name | What it is |
|------|------------|
| **Host kernel** | CEK decide logic (L1 role) |
| **Host runtime** | Kernel + process: network, DB, keys, config |
| **Peer kernel** | CEK apply logic (L1 role) |
| **Peer runtime** | Kernel + process: network, DOM/kv/device drivers |

This repo designs those kernels (and later crates). On a machine, kernels run **inside** the Host/Peer processes.

---

## Where they sit in the flow

```text
L7 app
  │  submit(Intent, Cap)
  ▼
Host runtime
  │  calls into
  ▼
Host kernel              ← sits here (authority path)
  │  Result{Ops}
  ▼
(transport / app forwards)
  │
  ▼
Peer runtime
  │  calls into
  ▼
Peer kernel              ← sits here (apply path)
  │  optional receipt
  ▼
Host runtime → Host kernel (lineage annotate / later reverse)
```

| Connection | How |
|------------|-----|
| **On the wire** | Host *runtime* ↔ Peer *runtime* (or app in the middle), carrying **contract** messages |
| **In-process** | Runtime code calls kernel APIs (`submit`, `apply`) as functions |

---

## What does not sit in the middle

```text
❌  App → “cek-runtime service” → Host kernel → Peer kernel
```

No required broker kernel. An optional message bus only **moves** `Result` / `receipt`; it is not Cap authority.

---

## Wire payloads (contract)

| Direction | Payload |
|-----------|---------|
| Caller → Host | Intent + Cap |
| Host → Caller / Peer | Result (`ops[]` or refusal/error) |
| Peer → Host | optional receipt |
| Host → Peer (reverse) | Result with inverse/restore Ops (or app pulls and applies) |

Handshake: **manifest** (`law_generation`, profiles, fail-closed). Missing → Baseline-only Peer.

---

## Three delivery patterns

1. **Caller-mediated** — app calls Host, then Peer with the same `Result`  
2. **Host pushes** — Host calls Peer apply after submit  
3. **Queue/bus** — contract messages on a topic (bus is not a kernel)  

Same kernels; only who forwards `Result` changes.

---

## One line

**Host kernel lives inside the Host runtime; Peer kernel lives inside the Peer runtime. They connect by contract messages between those runtimes (or by in-process calls).**
