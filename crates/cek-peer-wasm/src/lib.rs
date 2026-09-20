//! `hop: WASM C ABI + own JSON → peer kernel`.
//!
//! There is **no mint**. JSON serde lives here; apply is
//! [`cek_peer_kernel::apply_world`] → [`cek_peer_kernel::Peer::apply`].
//! This crate does **not** depend on `cek-peer-rust` (#10 wrong-owner).

#![cfg_attr(not(target_arch = "wasm32"), forbid(unsafe_code))]
#![deny(missing_docs)]

use cek_contract::{Receipt, ResultMsg};
use cek_peer_kernel::{apply_world, unknown_op_policy_from_wire, ApplyProfileKind};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Apply request (JSON). Same fields the TS runner understands.
#[derive(Debug, Clone, Deserialize)]
pub struct ApplyRequest {
    /// Host Result to apply.
    pub result: ResultMsg,
    /// `baseline` (default) or `ui`.
    #[serde(default)]
    pub profile: Option<String>,
    /// `skip` (default) or `fail_batch`.
    #[serde(default)]
    pub unknown_op_policy: Option<String>,
}

/// Apply response: receipt + world (for vector checks).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyResponse {
    /// Landed / failed Ops.
    pub receipt: Receipt,
    /// kv after apply.
    pub kv: BTreeMap<String, Value>,
    /// UI targets after apply.
    pub ui: BTreeMap<String, Value>,
    /// log lines after apply.
    pub log: Vec<String>,
}

/// Apply a JSON request body. Never mints. Failures return an error string.
pub fn apply_json(input: &str) -> Result<String, String> {
    let req: ApplyRequest =
        serde_json::from_str(input).map_err(|e| format!("request json: {e}"))?;
    let resp = apply_request(&req);
    serde_json::to_string(&resp).map_err(|e| format!("response json: {e}"))
}

/// Apply a typed request. Serde-owned types only; engine is the peer kernel helper.
pub fn apply_request(req: &ApplyRequest) -> ApplyResponse {
    let world = apply_world(
        &req.result,
        ApplyProfileKind::from_wire(req.profile.as_deref()),
        unknown_op_policy_from_wire(req.unknown_op_policy.as_deref()),
    );
    ApplyResponse {
        receipt: world.receipt,
        kv: world.kv,
        ui: world.ui,
        log: world.log,
    }
}

// ---- wasm32 C ABI (no wasm-bindgen) ---------------------------------------

#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;

#[cfg(target_arch = "wasm32")]
thread_local! {
    static LAST: RefCell<Vec<u8>> = RefCell::new(Vec::new());
}

/// Allocate `n` bytes in WASM memory. Caller writes UTF-8 request here.
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn cek_alloc(n: u32) -> *mut u8 {
    let mut v = vec![0u8; n as usize];
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p
}

/// Apply JSON at `ptr`/`len`. Returns result length (>=0) or -1 on error.
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn cek_apply(ptr: *const u8, len: u32) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len as usize) };
    let input = match std::str::from_utf8(slice) {
        Ok(s) => s,
        Err(_) => return -1,
    };
    let out = match apply_json(input) {
        Ok(s) => s.into_bytes(),
        Err(e) => e.into_bytes(),
    };
    let n = out.len() as i32;
    LAST.with(|c| *c.borrow_mut() = out);
    n
}

/// Pointer to last apply result bytes.
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn cek_result_ptr() -> *const u8 {
    LAST.with(|c| c.borrow().as_ptr())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cek_peer_kernel::ApplyWorld;

    fn fixture_kv_set() -> serde_json::Value {
        serde_json::json!({
            "result": {
                "kind": "ok",
                "ops": [{ "ns": "kv", "name": "set", "payload": { "key": "a", "value": 1 } }]
            },
            "profile": "baseline"
        })
    }

    fn fixture_refuse() -> serde_json::Value {
        serde_json::json!({
            "result": { "kind": "authority_refusal", "ops": [], "error": "no" }
        })
    }

    fn fixture_ui_morph() -> serde_json::Value {
        serde_json::json!({
            "result": {
                "kind": "ok",
                "ops": [{
                    "ns": "ui.dom",
                    "name": "morph",
                    "payload": { "target": "hdr", "patch": { "t": "new" } }
                }]
            },
            "profile": "ui"
        })
    }

    fn fixture_fail_batch() -> serde_json::Value {
        serde_json::json!({
            "result": {
                "kind": "ok",
                "ops": [
                    { "ns": "ui.dom", "name": "morph", "payload": {} },
                    { "ns": "kv", "name": "set", "payload": { "key": "a", "value": 1 } }
                ]
            },
            "unknown_op_policy": "fail_batch"
        })
    }

    fn fixture_ui_fail_batch() -> serde_json::Value {
        serde_json::json!({
            "result": {
                "kind": "ok",
                "ops": [
                    { "ns": "nope", "name": "x", "payload": {} },
                    { "ns": "kv", "name": "set", "payload": { "key": "a", "value": 1 } }
                ]
            },
            "profile": "ui",
            "unknown_op_policy": "fail_batch"
        })
    }

    #[test]
    fn apply_json_kv_set() {
        let out: ApplyResponse =
            serde_json::from_str(&apply_json(&fixture_kv_set().to_string()).unwrap()).unwrap();
        assert_eq!(out.receipt.landed.len(), 1);
        assert_eq!(out.kv.get("a"), Some(&serde_json::json!(1)));
    }

    #[test]
    fn refuse_is_noop() {
        let out: ApplyResponse =
            serde_json::from_str(&apply_json(&fixture_refuse().to_string()).unwrap()).unwrap();
        assert!(out.kv.is_empty());
        assert!(out.receipt.landed.is_empty());
    }

    #[test]
    fn json_path_matches_kernel_helper() {
        for body in [
            fixture_kv_set(),
            fixture_refuse(),
            fixture_ui_morph(),
            fixture_fail_batch(),
            fixture_ui_fail_batch(),
        ] {
            let via_json: ApplyResponse =
                serde_json::from_str(&apply_json(&body.to_string()).unwrap()).unwrap();
            let req: ApplyRequest = serde_json::from_value(body).unwrap();
            let world: ApplyWorld = apply_world(
                &req.result,
                ApplyProfileKind::from_wire(req.profile.as_deref()),
                unknown_op_policy_from_wire(req.unknown_op_policy.as_deref()),
            );
            assert_eq!(via_json.receipt, world.receipt);
            assert_eq!(via_json.kv, world.kv);
            assert_eq!(via_json.ui, world.ui);
            assert_eq!(via_json.log, world.log);
        }
    }

    #[test]
    fn ui_fail_batch_unknown_op_aborts_rest() {
        let out: ApplyResponse =
            serde_json::from_str(&apply_json(&fixture_ui_fail_batch().to_string()).unwrap())
                .unwrap();
        assert_eq!(out.receipt.failed.len(), 2);
        assert!(out.receipt.landed.is_empty());
        assert!(!out.kv.contains_key("a"));
    }
}
