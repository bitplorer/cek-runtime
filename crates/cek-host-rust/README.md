# cek-host-rust

Native Rust host runtime. Thin JSON mint/submit wire on `cek-host-kernel`.

Decide stays `Host::mint` / `Host::submit_for`. This crate owns **its own**
JSON port (`HostRuntime::host_json`). Sessionful: stores/keys/clock live on
the kernel. Not a second decide engine. Not a Python Host twin.

`cek host-json` hops onto this crate. Published Cap machine stays
`pip install cek-host`.

JSON: `{cmd:mint|submit, ...}` → Cap or Result.

```toml
cek-host-rust = "0.1.3"
```
