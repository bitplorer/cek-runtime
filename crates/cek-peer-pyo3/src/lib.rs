//! Apply-only Peer surface for in-process Python (PyO3).
//!
//! There is **no mint**. Callers pass the same Host `Result` JSON documents
//! as `cek-peer-rust` / `cek apply`; this crate hops onto that native
//! peer door and returns a receipt plus world snapshots.
//!
//! Lifecycle is explicit: **construct → bind → apply → release**. Import
//! only registers the module. One release door.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use cek_peer_rust::{ApplyRequest, ApplyResponse};

/// Re-export the shared JSON request (do not invent a parallel protocol).
pub use cek_peer_rust::{
    apply_json as rust_apply_json, apply_request, ApplyRequest as RustApplyRequest,
    ApplyResponse as RustApplyResponse,
};

/// Session state for the apply-only ABI handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbiState {
    /// Allocated; not yet bound. No kernel work.
    Constructed,
    /// Bound to this host process. Apply is allowed.
    Bound,
    /// Released. Apply is refused. Re-bind is refused.
    Released,
}

/// In-process apply-only ABI handle. Does not own a Peer world.
///
/// Each [`PeerAbi::apply_json`] calls `cek-peer-rust` (`apply_world` →
/// `Peer::apply` inside the kernel). Worlds do not accumulate.
pub struct PeerAbi {
    state: AbiState,
}

impl PeerAbi {
    /// Allocate a handle. No kernel work, no world.
    pub fn construct() -> Self {
        Self {
            state: AbiState::Constructed,
        }
    }

    /// Bind the handle so apply may run. Not Host Cap bind.
    pub fn bind(&mut self) -> Result<(), String> {
        match self.state {
            AbiState::Constructed => {
                self.state = AbiState::Bound;
                Ok(())
            }
            AbiState::Bound => Err("already bound".into()),
            AbiState::Released => Err("released".into()),
        }
    }

    /// Current lifecycle state.
    pub fn state(&self) -> AbiState {
        self.state
    }

    /// Apply a rust-door JSON document. Requires bind. Never mints.
    pub fn apply_json(&self, input: &str) -> Result<String, String> {
        self.require_bound()?;
        cek_peer_rust::apply_json(input)
    }

    /// Apply a typed rust-door request. Requires bind. Never mints.
    pub fn apply_request(&self, req: &ApplyRequest) -> Result<ApplyResponse, String> {
        self.require_bound()?;
        Ok(cek_peer_rust::apply_request(req))
    }

    /// The only cleanup door. Idempotent.
    pub fn release(&mut self) {
        self.state = AbiState::Released;
    }

    fn require_bound(&self) -> Result<(), String> {
        match self.state {
            AbiState::Bound => Ok(()),
            AbiState::Constructed => Err("not bound".into()),
            AbiState::Released => Err("released".into()),
        }
    }
}

impl Drop for PeerAbi {
    fn drop(&mut self) {
        self.release();
    }
}

/// Map a mutex lock. Poison is a distinct failure from post-release.
#[cfg(any(test, feature = "python"))]
pub(crate) fn map_mutex_lock<T>(r: std::sync::LockResult<T>) -> Result<T, String> {
    r.map_err(|_| "poisoned".into())
}

#[cfg(feature = "python")]
mod python;

#[cfg(test)]
mod tests {
    use super::*;
    use cek_contract::ResultMsg;
    use cek_peer_kernel::{apply_world, unknown_op_policy_from_wire, ApplyProfileKind, ApplyWorld};
    use serde_json::{json, Value};
    use std::path::PathBuf;
    use std::process::{Command, Stdio};

    fn bound() -> PeerAbi {
        let mut abi = PeerAbi::construct();
        abi.bind().unwrap();
        abi
    }

    fn parse_resp(s: &str) -> ApplyResponse {
        serde_json::from_str(s).unwrap()
    }

    fn apply_world_from_req(req: &ApplyRequest) -> ApplyWorld {
        apply_world(
            &req.result,
            ApplyProfileKind::from_wire(req.profile.as_deref()),
            unknown_op_policy_from_wire(req.unknown_op_policy.as_deref()),
        )
    }

    fn assert_parity(input: &str) {
        let mut abi = bound();
        let via_abi = parse_resp(&abi.apply_json(input).unwrap());
        abi.release();

        let via_rust_apply_json = parse_resp(&cek_peer_rust::apply_json(input).unwrap());
        assert_eq!(
            serde_json::to_value(&via_abi).unwrap(),
            serde_json::to_value(&via_rust_apply_json).unwrap(),
            "PyO3 wrapper must match cek-peer-rust::apply_json"
        );

        let req: ApplyRequest = serde_json::from_str(input).unwrap();
        let world = apply_world_from_req(&req);
        assert_eq!(via_abi.receipt, world.receipt);
        assert_eq!(via_abi.kv, world.kv);
        assert_eq!(via_abi.ui, world.ui);
        assert_eq!(via_abi.log, world.log);
    }

    #[test]
    fn construct_does_no_kernel_work() {
        let abi = PeerAbi::construct();
        assert_eq!(abi.state(), AbiState::Constructed);
        assert!(abi.apply_json("{}").is_err());
    }

    #[test]
    fn bind_then_release_is_one_door() {
        let mut abi = PeerAbi::construct();
        abi.bind().unwrap();
        assert_eq!(abi.state(), AbiState::Bound);
        abi.release();
        assert_eq!(abi.state(), AbiState::Released);
        abi.release();
        assert_eq!(abi.state(), AbiState::Released);
        assert_eq!(abi.apply_json("{}").unwrap_err(), "released");
        assert_eq!(abi.bind().unwrap_err(), "released");
    }

    #[test]
    fn mutex_poison_is_not_released() {
        let m = std::sync::Mutex::new(0u8);
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _g = m.lock().unwrap();
            panic!("poison");
        }));
        let err = super::map_mutex_lock(m.lock()).unwrap_err();
        assert_eq!(err, "poisoned");
        assert_ne!(err, "released");
    }

    #[test]
    fn python_lock_poison_does_not_say_released() {
        let src = include_str!("python.rs");
        assert!(
            !src.contains("new_err(\"released\")"),
            "PyO3 mutex poison must not be reported as released"
        );
        assert!(
            src.contains("map_mutex_lock"),
            "PyO3 lock path must distinguish poison via map_mutex_lock"
        );
    }

    #[test]
    fn apply_json_kv_set() {
        let req = json!({
            "result": {
                "kind": "ok",
                "ops": [{ "ns": "kv", "name": "set", "payload": { "key": "a", "value": 1 } }]
            },
            "profile": "baseline"
        });
        assert_parity(&req.to_string());
        let out = parse_resp(&bound().apply_json(&req.to_string()).unwrap());
        assert_eq!(out.receipt.landed.len(), 1);
        assert_eq!(out.kv.get("a"), Some(&json!(1)));
    }

    #[test]
    fn apply_json_kv_delete() {
        let req = json!({
            "result": {
                "kind": "ok",
                "ops": [
                    { "ns": "kv", "name": "set", "payload": { "key": "a", "value": 1 } },
                    { "ns": "kv", "name": "delete", "payload": { "key": "a" } }
                ]
            },
            "profile": "baseline"
        });
        assert_parity(&req.to_string());
        let out = parse_resp(&bound().apply_json(&req.to_string()).unwrap());
        assert_eq!(out.receipt.landed.len(), 2);
        assert!(out.kv.get("a").is_none());
    }

    #[test]
    fn apply_json_log_append() {
        let req = json!({
            "result": {
                "kind": "ok",
                "ops": [{ "ns": "log", "name": "append", "payload": { "message": "hello" } }]
            },
            "profile": "baseline"
        });
        assert_parity(&req.to_string());
        let out = parse_resp(&bound().apply_json(&req.to_string()).unwrap());
        assert_eq!(out.receipt.landed.len(), 1);
        assert_eq!(out.log, vec!["hello".to_string()]);
    }

    #[test]
    fn apply_json_ui_dom_and_kv_batch() {
        let req = json!({
            "result": {
                "kind": "ok",
                "ops": [
                    { "ns": "kv", "name": "set", "payload": { "key": "a", "value": 1 } },
                    {
                        "ns": "ui.dom",
                        "name": "morph",
                        "payload": { "target": "hdr", "patch": { "t": "n" }, "snapshot": { "t": "o" } }
                    }
                ]
            },
            "profile": "ui"
        });
        assert_parity(&req.to_string());
        let out = parse_resp(&bound().apply_json(&req.to_string()).unwrap());
        assert_eq!(out.receipt.landed.len(), 2);
        assert_eq!(out.kv.get("a"), Some(&json!(1)));
        assert_eq!(out.ui.get("hdr"), Some(&json!({"t": "n"})));
    }

    #[test]
    fn apply_json_ui_fail_batch_unknown_op_aborts_rest() {
        let req = json!({
            "result": {
                "kind": "ok",
                "ops": [
                    { "ns": "nope", "name": "x", "payload": {} },
                    { "ns": "kv", "name": "set", "payload": { "key": "a", "value": 1 } }
                ]
            },
            "profile": "ui",
            "unknown_op_policy": "fail_batch"
        });
        assert_parity(&req.to_string());
        let out = parse_resp(&bound().apply_json(&req.to_string()).unwrap());
        assert_eq!(out.receipt.failed.len(), 2);
        assert!(out.receipt.landed.is_empty());
        assert!(!out.kv.contains_key("a"));
    }

    #[test]
    fn refuse_is_noop() {
        let req = json!({
            "result": { "kind": "authority_refusal", "ops": [], "error": "no" }
        });
        assert_parity(&req.to_string());
        let out = parse_resp(&bound().apply_json(&req.to_string()).unwrap());
        assert!(out.kv.is_empty());
        assert!(out.receipt.landed.is_empty());
    }

    #[test]
    fn each_apply_is_a_fresh_world() {
        let abi = bound();
        let set = json!({
            "result": {
                "kind": "ok",
                "ops": [{ "ns": "kv", "name": "set", "payload": { "key": "a", "value": 1 } }]
            }
        });
        let first = parse_resp(&abi.apply_json(&set.to_string()).unwrap());
        assert_eq!(first.kv.get("a"), Some(&json!(1)));
        let empty = json!({ "result": { "kind": "ok", "ops": [] } });
        let second = parse_resp(&abi.apply_json(&empty.to_string()).unwrap());
        assert!(second.kv.is_empty());
    }

    fn vector_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../crates/cek-contract/vectors")
    }

    #[test]
    fn peer_result_vectors_match_kernel_and_rust() {
        let dir = vector_dir();
        let mut seen = 0;
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let c: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            if c.get("peer_result").is_none() {
                continue;
            }
            seen += 1;
            let req = json!({
                "result": c["peer_result"],
                "profile": if c.get("peer_profile").and_then(|v| v.as_str()) == Some("ui") {
                    "ui"
                } else {
                    "baseline"
                },
                "unknown_op_policy": if c.get("peer_unknown_policy").and_then(|v| v.as_str())
                    == Some("fail_batch")
                {
                    "fail_batch"
                } else {
                    "skip"
                }
            });
            assert_parity(&req.to_string());
            let out = parse_resp(&bound().apply_json(&req.to_string()).unwrap());
            if let Some(expect) = c.get("expect_peer_kv") {
                for (k, v) in expect.as_object().unwrap() {
                    if v.is_null() {
                        assert!(
                            !out.kv.contains_key(k),
                            "{} kv[{k}] should be absent",
                            c["id"]
                        );
                    } else {
                        assert_eq!(out.kv.get(k), Some(v), "{} kv[{k}]", c["id"]);
                    }
                }
            }
            if let Some(expect) = c.get("expect_peer_ui") {
                for (k, v) in expect.as_object().unwrap() {
                    if v.is_null() {
                        assert!(
                            !out.ui.contains_key(k),
                            "{} ui[{k}] should be absent",
                            c["id"]
                        );
                    } else {
                        assert_eq!(out.ui.get(k), Some(v), "{} ui[{k}]", c["id"]);
                    }
                }
            }
        }
        assert!(seen >= 6, "expected peer_result fixtures, got {seen}");
    }

    fn workspace_cek() -> Option<PathBuf> {
        if let Ok(p) = std::env::var("CEK_BIN") {
            let p = PathBuf::from(p);
            if p.is_file() {
                return Some(p);
            }
        }
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        for rel in ["target/debug/cek", "target/release/cek"] {
            let cand = root.join(rel);
            if cand.is_file() {
                return Some(cand);
            }
        }
        None
    }

    #[test]
    fn apply_json_matches_cek_apply_subprocess_when_present() {
        let Some(bin) = workspace_cek() else {
            return;
        };
        let req = json!({
            "result": {
                "kind": "ok",
                "ops": [
                    { "ns": "kv", "name": "set", "payload": { "key": "a", "value": 1 } },
                    { "ns": "log", "name": "append", "payload": { "message": "hi" } },
                    {
                        "ns": "ui.dom",
                        "name": "morph",
                        "payload": { "target": "hdr", "patch": { "t": "n" }, "snapshot": { "t": "o" } }
                    }
                ]
            },
            "profile": "ui"
        });
        let body = req.to_string();
        let via_abi = parse_resp(&bound().apply_json(&body).unwrap());
        let mut child = Command::new(bin)
            .arg("apply")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        {
            use std::io::Write;
            child
                .stdin
                .as_mut()
                .unwrap()
                .write_all(body.as_bytes())
                .unwrap();
        }
        let out = child.wait_with_output().unwrap();
        assert!(
            out.status.success(),
            "cek apply: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let via_cli: ApplyResponse = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(
            serde_json::to_value(&via_abi).unwrap(),
            serde_json::to_value(&via_cli).unwrap()
        );
    }

    #[test]
    fn apply_unbound_is_refused() {
        let abi = PeerAbi::construct();
        assert_eq!(abi.apply_json("{}").unwrap_err(), "not bound");
        let req = ApplyRequest {
            result: ResultMsg::ok(vec![]),
            profile: None,
            unknown_op_policy: None,
        };
        assert_eq!(abi.apply_request(&req).unwrap_err(), "not bound");
    }
}
