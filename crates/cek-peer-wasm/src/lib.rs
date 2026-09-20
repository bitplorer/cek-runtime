//! WASM ABI hop onto the shared JSON apply door in `cek-peer-rust`.
//!
//! There is **no mint**. This crate does not own the apply/receipt contract.
//! Callers that want the JSON port use [`cek_peer_rust::apply_json`].

#![cfg_attr(not(target_arch = "wasm32"), forbid(unsafe_code))]
#![deny(missing_docs)]

/// Re-export the shared JSON door. Not a second apply contract.
pub use cek_peer_rust::{apply_json, apply_request, ApplyRequest, ApplyResponse};

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
    let out = match cek_peer_rust::apply_json(input) {
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
    fn hop_apply_json_kv_set() {
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
    fn hop_refuse_is_noop() {
        let req = serde_json::json!({
            "result": { "kind": "authority_refusal", "ops": [], "error": "no" }
        });
        let out: ApplyResponse =
            serde_json::from_str(&apply_json(&req.to_string()).unwrap()).unwrap();
        assert!(out.kv.is_empty());
        assert!(out.receipt.landed.is_empty());
    }
}
