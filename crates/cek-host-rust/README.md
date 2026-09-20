# cek-host-rust

`hop: native JSON → host kernel`

Decide stays `Host::mint` / `Host::submit_for` on `cek-host-kernel`.
This crate owns **its own** JSON port (`HostRuntime::host_json`).
Sessionful: stores/keys/clock live on the host kernel. Not a second
decide engine. Not a Python Host twin.

`cek host-json` hops onto this crate. Published Cap machine stays
`pip install cek-host`.

JSON: `{cmd:mint|submit, ...}` → Cap or Result.

Live on crates.io at **0.1.3**, alongside the peer hops.

```toml
cek-host-rust = "0.1.3"
```
