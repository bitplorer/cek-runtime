# Peer drivers

Drivers are the **world**. They sit **outside** the Peer kernel.

```text
Host kernel  →  Result{Ops}
Peer kernel  →  asks a driver to apply each Op
driver       →  mutates kv / DOM / log
```

A driver is **not** a kernel. It does not mint, verify, or refuse Caps.  
If the Result is `authority_refusal` or `dispatch_error`, the kernel never calls a driver.

Law: [cek-framework](https://github.com/bitplorer/cek-framework). Walkthrough: [GUIDE.md](GUIDE.md).

## Catalog

- **kv**
  - **Where:** `crates/cek-ops-baseline` (`KvStore`)
  - **Ops:** `kv.set`, `kv.delete`
  - **World:** map of key → JSON
- **log**
  - **Where:** Peer kernel (in-memory `Vec`)
  - **Ops:** `log.append`
  - **World:** append-only lines
- **ui (flat)**
  - **Where:** `crates/cek-ops-ui` (`UiStore`)
  - **Ops:** `ui.dom.morph`, `ui.dom.restore`
  - **World:** map of target → JSON
- **DOM (tree)**
  - **Where:** `crates/cek-ops-ui` (`DomTree`); JS `ports/cek-peer-js`
  - **Ops:** `ui.dom.morph`, `ui.dom.restore`
  - **World:** `{ tag, attrs, children, text? }`

Baseline Peers ship **kv + log** only. Unknown Ops follow profile policy (`skip` / `fail_batch`).  
A UI/DOM Peer is constructed with `Peer::with_ui()` (flat) or the JS `--profile dom` (tree).

## kv (`cek-ops-baseline`)

- **`kv.set`**
  - **Payload:** `{ key, value }`
  - **Effect:** write
- **`kv.delete`**
  - **Payload:** `{ key, prior? }`
  - **Effect:** remove. `prior` is **not** used at apply time — Host put it there for reverse

Host reverse:

- `kv.set` → `kv.delete` of that key  
- `kv.delete` with `prior` → `kv.set` of that prior  
- `kv.delete` without `prior` → honest **non-reversible**

Empty `key` is a Host **dispatch_error** (never reaches the driver).

## log

- **`log.append`**
  - **Payload:** `{ message }`
  - **Effect:** push a line

No inverse. Activity end marks **non-reversible**. Missing `message` is a Host dispatch_error.

## ui flat (`UiStore`)

Used by the Rust Peer `with_ui()` profile and by vectors that check `expect_peer_ui`.

- **`ui.dom.morph`**
  - **Payload:** `{ target, patch, snapshot? }`
  - **Effect:** `store[target] = patch`
- **`ui.dom.restore`**
  - **Payload:** `{ target, snapshot }`
  - **Effect:** `store[target] = snapshot`

`snapshot` is ignored at apply of morph. Host copies it onto the inverse `ui.dom.restore`.  
No snapshot → reverse is **non-reversible**.

## DOM tree (`DomTree`)

Same two Ops. The world is a forest of nodes:

```json
{ "tag": "div", "attrs": { "id": "root" }, "children": [], "text": null }
```

**Address a node** (all of these are the `target` string):

- **`root`** — `attrs.id == "root"`
- **`#root`** — same
- **`/0`** — first root
- **`/0/1`** — second child of the first root

**Driver helpers** (not Ops — tests and the JS runtime use them):

- **`morph` / `restore`** — replace the addressed node
- **`insert_child`** — append under a parent
- **`remove`** — take the node out; returns the snapshot
- **`set_text` / `set_attr`** — mutate in place
- **`html`** — string render (not a browser)
- **`by_id`** — deterministic id → node map

Missing target → apply **fails** that Op (receipt `failed`). Fail closed. The driver does not invent a node.

JS: `ports/cek-peer-js` `DomTree` matches this. Profile `dom` applies `ui.dom.*` onto the tree **and** the flat `ui` map (vectors still read the map).

## What a driver must never do

- Mint or verify a Cap  
- Turn a refusal Result into world changes  
- Widen scopes, attach signatures, or speak law generation  
- Treat `trace` as permission  

Those stay on the **Host kernel**.

## Adding a driver

1. New world crate next to `cek-ops-*` (or a port-local store).  
2. Apply only **Ops** the Host already projects.  
3. Do not add Host verify/mint to the driver.  
4. If you need a new Action, that is Host **project**, documented in [GUIDE.md](GUIDE.md) §5 — then the driver applies the new Op.

No third kernel. No `extensions/` layer.
