//! Dispatch Intent → authorized Ops, then project onto Result (LAW §11).
//!
//! Kernel actions only. `dispatch_ops` is LAW §4 step 3 (the mapper currently
//! named `project_ops`). `project_authorized` is LAW §4 step 5: Result Ops are
//! this Peer's ability ∪ Baseline. Missing Manifest → Baseline-only Peer.
//! Manifest never grants Cap.

use cek_contract::{baseline, Intent, Op, Profile};
use serde_json::json;

/// LAW §4 step 3: dispatch Intent → authorized Ops. Miss is an error string.
pub(crate) fn dispatch_ops(intent: &Intent) -> Result<Vec<Op>, String> {
    project_ops(intent)
}

/// True if `profile.apply_set` contains this Op by pair identity (not FQ concat).
fn ability_contains(profile: &Profile, op: &Op) -> bool {
    profile.apply_set.iter().any(|s| {
        cek_contract::Pair::from_fq(s)
            .map(|p| p.ns == op.ns && p.name == op.name)
            .unwrap_or(false)
    })
}

/// LAW §4 step 5 / LAW §11: project authorized Ops for the Result.
///
/// `Result.ops ⊆ peer.apply_set ∪ Baseline`. Ops the Peer cannot apply are
/// lowered via [`cek_contract::lower_to_baseline`] (Baseline fallback).
/// `profile == None` is a missing Manifest → Baseline-only Peer.
pub(crate) fn project_authorized(authorized: Vec<Op>, profile: Option<&Profile>) -> Vec<Op> {
    match profile {
        None => authorized
            .iter()
            .filter_map(cek_contract::lower_to_baseline)
            .collect(),
        Some(profile) => authorized
            .iter()
            .filter_map(|op| {
                if ability_contains(profile, op) {
                    Some(op.clone())
                } else {
                    cek_contract::lower_to_baseline(op)
                }
            })
            .collect(),
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use cek_contract::{ui_morph, UnknownOpPolicy, PROFILE_BASELINE, PROFILE_UI, UI_OPS};
    use serde_json::json;

    fn baseline_profile() -> Profile {
        Profile {
            name: PROFILE_BASELINE.into(),
            apply_set: cek_contract::BASELINE_OPS
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
            unknown_op_policy: UnknownOpPolicy::Skip,
        }
    }

    fn ui_profile() -> Profile {
        let mut apply: Vec<String> = cek_contract::BASELINE_OPS
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        apply.extend(UI_OPS.iter().map(|s| (*s).to_string()));
        Profile {
            name: PROFILE_UI.into(),
            apply_set: apply,
            unknown_op_policy: UnknownOpPolicy::Skip,
        }
    }

    fn morph() -> Op {
        ui_morph("hdr", json!({"t": 1}), Some(json!({"t": 0})))
    }

    #[test]
    fn missing_profile_lowers_ui_to_baseline() {
        let out = project_authorized(vec![morph()], None);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].fq(), "kv.set");
        assert!(out.iter().all(|op| op.ns != "ui.dom"));
    }

    #[test]
    fn baseline_profile_lowers_ui_not_full_catalog() {
        let out = project_authorized(vec![morph()], Some(&baseline_profile()));
        assert_eq!(out[0].fq(), "kv.set");
        assert!(out.iter().all(|op| op.fq() != "ui.dom.morph"));
    }

    #[test]
    fn ui_profile_keeps_domain_op() {
        let out = project_authorized(vec![morph()], Some(&ui_profile()));
        assert_eq!(out[0].fq(), "ui.dom.morph");
    }

    #[test]
    fn limited_apply_set_unions_baseline() {
        let limited = Profile {
            name: "limited".into(),
            apply_set: vec!["kv.set".into()],
            unknown_op_policy: UnknownOpPolicy::Skip,
        };
        let ops = vec![morph(), baseline::log_append("hi")];
        let out = project_authorized(ops, Some(&limited));
        assert!(out.iter().any(|op| op.fq() == "kv.set"));
        assert!(out.iter().any(|op| op.fq() == "log.append"));
        assert!(out.iter().all(|op| op.ns != "ui.dom"));
    }

    #[test]
    fn same_authorized_and_profile_is_deterministic() {
        let ops = vec![morph()];
        let a = project_authorized(ops.clone(), Some(&baseline_profile()));
        let b = project_authorized(ops, Some(&baseline_profile()));
        assert_eq!(a, b);
    }
}
