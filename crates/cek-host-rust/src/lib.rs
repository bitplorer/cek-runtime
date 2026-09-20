//! Native Rust host runtime: thin JSON mint/submit door on the kernel.
//!
//! This crate owns **this hop's** JSON wire (`HostRuntime::host_json`).
//! Decide stays [`cek_host_kernel::Host`]. Sessionful: stores/keys/clock
//! live on the kernel. Not a second decide engine. Not a Python Host twin.
//! Cap mint stays inside the kernel.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use cek_contract::{Intent, Profile};
use cek_host_kernel::Host;
use serde_json::json;

/// Sessionful Host runtime wrapping a kernel [`Host`].
pub struct HostRuntime {
    host: Host,
}

impl HostRuntime {
    /// Default in-memory Host (same construction as `cek host-json`).
    pub fn new() -> Self {
        Self { host: Host::new() }
    }

    /// Wrap an existing kernel Host (tests / custom clock or keys).
    pub fn wrap(host: Host) -> Self {
        Self { host }
    }

    /// JSON door. Frozen cmds: `mint` | `submit`.
    ///
    /// mint:   `{"cmd":"mint","id":"...","action":"kv.write","once":false}`
    /// submit: `{"cmd":"submit","intent":{...},"profile":{...}?}`
    ///         omitted profile → missing Manifest → Baseline-only (LAW §11).
    pub fn host_json(&self, input: &str) -> Result<String, String> {
        let v: serde_json::Value = serde_json::from_str(input).map_err(|e| format!("json: {e}"))?;
        let cmd = v.get("cmd").and_then(|c| c.as_str()).unwrap_or("");
        match cmd {
            "mint" => {
                let id = v.get("id").and_then(|x| x.as_str()).unwrap_or("cap");
                let action = v.get("action").and_then(|x| x.as_str()).unwrap_or("");
                let once = v.get("once").and_then(|x| x.as_bool()).unwrap_or(false);
                let cap = self.host.mint(id, action, once, None);
                serde_json::to_string(&cap).map_err(|e| e.to_string())
            }
            "submit" => {
                let intent: Intent =
                    serde_json::from_value(v.get("intent").cloned().unwrap_or(json!({})))
                        .map_err(|e| format!("intent: {e}"))?;
                let profile: Option<Profile> = match v.get("profile") {
                    None | Some(serde_json::Value::Null) => None,
                    Some(p) => Some(
                        serde_json::from_value(p.clone()).map_err(|e| format!("profile: {e}"))?,
                    ),
                };
                let result = self.host.submit_for(intent, profile.as_ref());
                serde_json::to_string(&result).map_err(|e| e.to_string())
            }
            _ => Err("cmd must be mint|submit".into()),
        }
    }
}

impl Default for HostRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cek_contract::{Cap, ResultKind, ResultMsg};
    use serde_json::json;

    fn mint_body(id: &str, action: &str, once: bool) -> String {
        json!({"cmd": "mint", "id": id, "action": action, "once": once}).to_string()
    }

    fn submit_body(cap: &Cap, action: &str) -> String {
        json!({
            "cmd": "submit",
            "intent": {
                "action": action,
                "args": { "key": "greeting", "value": "hello" },
                "cap": cap
            }
        })
        .to_string()
    }

    #[test]
    fn mint_json_returns_cap() {
        let host = HostRuntime::new();
        let cap: Cap = serde_json::from_str(
            &host
                .host_json(&mint_body("cap-ok", "kv.write", false))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(cap.id, "cap-ok");
        assert_eq!(cap.action, "kv.write");
        assert!(!cap.once);
    }

    #[test]
    fn submit_json_ok_after_mint_on_same_session() {
        let host = HostRuntime::new();
        let cap: Cap = serde_json::from_str(
            &host
                .host_json(&mint_body("cap-ok", "kv.write", false))
                .unwrap(),
        )
        .unwrap();
        let result: ResultMsg =
            serde_json::from_str(&host.host_json(&submit_body(&cap, "kv.write")).unwrap()).unwrap();
        assert_eq!(result.kind, ResultKind::Ok);
        assert_eq!(result.ops.len(), 1);
        assert_eq!(result.ops[0].fq(), "kv.set");
    }

    #[test]
    fn submit_json_refuses_action_mismatch() {
        let host = HostRuntime::new();
        let cap: Cap = serde_json::from_str(
            &host
                .host_json(&mint_body("cap-bad", "kv.read", false))
                .unwrap(),
        )
        .unwrap();
        let result: ResultMsg =
            serde_json::from_str(&host.host_json(&submit_body(&cap, "kv.write")).unwrap()).unwrap();
        assert_eq!(result.kind, ResultKind::AuthorityRefusal);
        assert!(result.ops.is_empty());
    }

    #[test]
    fn unknown_cmd_uses_frozen_error() {
        let err = HostRuntime::new()
            .host_json(r#"{"cmd":"receipt"}"#)
            .unwrap_err();
        assert_eq!(err, "cmd must be mint|submit");
    }

    #[test]
    fn bad_json_uses_frozen_prefix() {
        let err = HostRuntime::new().host_json("not-json").unwrap_err();
        assert!(err.starts_with("json: "), "{err}");
    }

    #[test]
    fn mint_defaults_match_host_json_glue() {
        let host = HostRuntime::new();
        let cap: Cap = serde_json::from_str(&host.host_json(r#"{"cmd":"mint"}"#).unwrap()).unwrap();
        assert_eq!(cap.id, "cap");
        assert_eq!(cap.action, "");
        assert!(!cap.once);
    }

    #[test]
    fn json_path_matches_kernel_mint_and_submit() {
        let kernel = cek_host_kernel::Host::with_clock(1_000);
        let runtime = HostRuntime::wrap(cek_host_kernel::Host::with_clock(1_000));
        let via_json: Cap = serde_json::from_str(
            &runtime
                .host_json(&mint_body("cap-par", "kv.write", false))
                .unwrap(),
        )
        .unwrap();
        let via_kernel = kernel.mint("cap-par", "kv.write", false, None);
        assert_eq!(via_json, via_kernel);

        let via_json: ResultMsg = serde_json::from_str(
            &runtime
                .host_json(&submit_body(&via_json, "kv.write"))
                .unwrap(),
        )
        .unwrap();
        let via_kernel = kernel.submit_for(
            serde_json::from_value(json!({
                "action": "kv.write",
                "args": { "key": "greeting", "value": "hello" },
                "cap": via_kernel
            }))
            .unwrap(),
            None,
        );
        assert_eq!(via_json, via_kernel);
    }
}
