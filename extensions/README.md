# Extensions — not kernel

Anything the CEK **kernel** does not need lives here. See [LAYERS.md](../LAYERS.md).

| Crate | Side | Job |
|-------|------|-----|
| [`cek-ext-ui`](cek-ext-ui) | Host | `UiPack` projects `ui.morph`. Optional `DomTree`. |
| [`cek-ops-ui`](cek-ops-ui) | Peer | `UiStore` + `apply_op` for `ui.dom.*`. |

Neither crate mints Caps. Register the pack with `Host::with_pack`. Use `Peer::with_ui()` for the world.
