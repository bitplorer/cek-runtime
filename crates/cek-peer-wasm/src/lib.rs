//! Apply-only Peer surface for WASM and other ports.
//!
//! There is **no mint**. Callers pass a Host `Result` as JSON; this crate
//! applies it and returns a receipt plus world snapshots.

#![cfg_attr(not(target_arch = "wasm32"), forbid(unsafe_code))]
#![deny(missing_docs)]

use cek_contract::{Receipt, ResultMsg, UnknownOpPolicy};
use cek_peer_kernel::Peer;
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

/// Apply a typed request (native + WASM).
pub fn apply_request(req: &ApplyRequest) -> ApplyResponse {
    let policy = match req.unknown_op_policy.as_deref() {
        Some("fail_batch") => UnknownOpPolicy::FailBatch,
        _ => UnknownOpPolicy::Skip,
    };
    let peer = if req.profile.as_deref() == Some("ui") {
        Peer::with_ui()
    } else {
        Peer::with_policy(policy)
    };
    let receipt = peer.apply(&req.result).unwrap_or(Receipt {
        landed: Vec::new(),
        failed: Vec::new(),
    });
    ApplyResponse {
        receipt,
        kv: peer.kv_snapshot(),
        ui: peer.ui_snapshot(),
        log: peer.log_lines(),
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

    #[test]
    fn apply_json_kv_set() {
        let req = serde_json::json!({
            "result": {
                "kind": "ok",
                "ops": [{ "ns": "kv", "name": "set", "payload": { "key": "a", "value": 1 } }]
            },
            "profile": "baseline"
        });
        let out: ApplyResponse =
            serde_json::from_str(&apply_json(&req.to_string()).unwrap()).unwrap();
        assert_eq!(out.receipt.landed.len(), 1);
        assert_eq!(out.kv.get("a"), Some(&serde_json::json!(1)));
    }

    #[test]
    fn refuse_is_noop() {
        let req = serde_json::json!({
            "result": { "kind": "authority_refusal", "ops": [], "error": "no" }
        });
        let out: ApplyResponse =
            serde_json::from_str(&apply_json(&req.to_string()).unwrap()).unwrap();
        assert!(out.kv.is_empty());
        assert!(out.receipt.landed.is_empty());
    }
}
