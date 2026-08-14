# Extensions — not kernel

Anything the CEK **kernel** does not need lives here.

| May live here | Must not live here |
|---------------|-------------------|
| Domain packs (`ui.*`, future packs) | Cap mint / verify |
| Optional worlds (JSON map, `DomTree`) | BoundAsk |
| Extra Peer stores / profiles | Once / idempotency / refuse path |
| Signing helpers used only by a pack | Law changes |

Kernel Baseline is `kv.*` + `log.*` only. Register a pack with `Host::with_pack`.

- [`cek-ext-ui`](cek-ext-ui): `UiPack` projects `ui.morph`. `DomTree` is an optional tree world.
