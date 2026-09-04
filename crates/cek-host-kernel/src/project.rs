//! Dispatch Intent → authorized Ops, then first-cut project onto Result.
//!
//! Kernel actions only. `dispatch_ops` is LAW §4 step 3 (the mapper currently
//! named `project_ops`). `project_authorized` is LAW §4 step 5 (identity;
//! profile negotiate / Baseline lower on submit is out of scope).

use cek_contract::{baseline, Intent, Op};
use serde_json::json;

/// LAW §4 step 3: dispatch Intent → authorized Ops. Miss is an error string.
pub(crate) fn dispatch_ops(intent: &Intent) -> Result<Vec<Op>, String> {
    project_ops(intent)
}

/// LAW §4 step 5: project authorized Ops for the Result.
///
/// First-cut: identity. Peer-profile negotiate / Baseline lower is not done here.
pub(crate) fn project_authorized(authorized: Vec<Op>) -> Vec<Op> {
    authorized
}

pub(crate) fn project_baseline(intent: &Intent) -> Result<Vec<Op>, String> {
    match intent.action.as_str() {
        cek_contract::ACTION_KV_WRITE => {
            let key = intent
                .args
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "kv.write requires string args.key".to_string())?;
            if key.is_empty() {
                return Err("kv.write key must be non-empty".into());
            }
            let value = intent.args.get("value").cloned().unwrap_or(json!(null));
            Ok(vec![baseline::kv_set(key, value)])
        }
        cek_contract::ACTION_KV_DELETE => {
            let key = intent
                .args
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "kv.delete requires string args.key".to_string())?;
            if key.is_empty() {
                return Err("kv.delete key must be non-empty".into());
            }
            let prior = intent.args.get("prior").cloned();
            Ok(vec![baseline::kv_delete_prior(key, prior)])
        }
        cek_contract::ACTION_LOG_APPEND => {
            let msg = intent
                .args
                .get("message")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "log.append requires string args.message".to_string())?;
            Ok(vec![baseline::log_append(msg)])
        }
        other => Err(format!("unknown action: {other}")),
    }
}

pub(crate) fn project_ops(intent: &Intent) -> Result<Vec<Op>, String> {
    match intent.action.as_str() {
        cek_contract::ACTION_UI_MORPH => {
            let target = intent
                .args
                .get("target")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "ui.morph requires string args.target".to_string())?;
            if target.is_empty() {
                return Err("ui.morph target must be non-empty".into());
            }
            let patch = intent
                .args
                .get("patch")
                .cloned()
                .ok_or_else(|| "ui.morph requires args.patch".to_string())?;
            let snapshot = intent.args.get("snapshot").cloned();
            Ok(vec![cek_contract::ui_morph(target, patch, snapshot)])
        }
        cek_contract::ACTION_UI_RESTORE => {
            let target = intent
                .args
                .get("target")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "ui.restore requires string args.target".to_string())?;
            if target.is_empty() {
                return Err("ui.restore target must be non-empty".into());
            }
            let snapshot = intent
                .args
                .get("snapshot")
                .cloned()
                .ok_or_else(|| "ui.restore requires args.snapshot".to_string())?;
            Ok(vec![cek_contract::ui_restore(target, snapshot)])
        }
        _ => project_baseline(intent),
    }
}

pub(crate) fn inverse_ops(ops: &[Op]) -> Vec<Op> {
    let mut inv = Vec::new();
    for op in ops.iter().rev() {
        if let Some(kv_inv) = cek_contract::inverse_kv(op) {
            inv.push(kv_inv);
        } else if let Some(ui_inv) = cek_contract::inverse_ui(op) {
            inv.push(ui_inv);
        }
    }
    inv
}
