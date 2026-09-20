//! One-shot typed apply helper.
//!
//! Hops own JSON (`serde` only). This module owns **profile/policy →
//! [`Peer::apply`](crate::Peer::apply) → receipt + snapshots**. There is
//! still one engine: [`Peer::apply`](crate::Peer::apply). No mint. No
//! second apply contract.

use crate::Peer;
use cek_contract::{Receipt, ResultMsg, UnknownOpPolicy};
use serde_json::Value;
use std::collections::BTreeMap;

/// Apply-set a hop selects for a one-shot apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyProfileKind {
    /// `kv.*` + `log.*` (JSON `profile` default / `"baseline"`).
    Baseline,
    /// Baseline + `ui.dom.*` (JSON `profile` `"ui"`).
    Ui,
}

impl ApplyProfileKind {
    /// JSON `profile` field: `"ui"` → [`Self::Ui`]; anything else → [`Self::Baseline`].
    pub fn from_wire(profile: Option<&str>) -> Self {
        match profile {
            Some("ui") => Self::Ui,
            _ => Self::Baseline,
        }
    }
}

/// JSON `unknown_op_policy`: `"fail_batch"` → [`UnknownOpPolicy::FailBatch`]; else Skip.
pub fn unknown_op_policy_from_wire(policy: Option<&str>) -> UnknownOpPolicy {
    match policy {
        Some("fail_batch") => UnknownOpPolicy::FailBatch,
        _ => UnknownOpPolicy::Skip,
    }
}

/// Receipt plus world snapshots after one [`Peer::apply`](crate::Peer::apply).
#[derive(Debug, Clone, PartialEq)]
pub struct ApplyWorld {
    /// Landed / failed Ops.
    pub receipt: Receipt,
    /// kv after apply.
    pub kv: BTreeMap<String, Value>,
    /// UI targets after apply.
    pub ui: BTreeMap<String, Value>,
    /// log lines after apply.
    pub log: Vec<String>,
}

/// Build a Peer from profile/policy, apply once, return receipt + snapshots.
///
/// UI profile uses [`Peer::with_ui_policy`](crate::Peer::with_ui_policy).
/// Baseline uses [`Peer::with_policy`](crate::Peer::with_policy). Engine is
/// always [`Peer::apply`](crate::Peer::apply).
pub fn apply_world(
    result: &ResultMsg,
    profile: ApplyProfileKind,
    unknown_op_policy: UnknownOpPolicy,
) -> ApplyWorld {
    let peer = match profile {
        ApplyProfileKind::Ui => Peer::with_ui_policy(unknown_op_policy),
        ApplyProfileKind::Baseline => Peer::with_policy(unknown_op_policy),
    };
    let receipt = peer.apply(result).expect(
        "one-shot apply_world: Peer::apply returned None (single-flight); \
         a freshly constructed Peer cannot be in-flight",
    );
    ApplyWorld {
        receipt,
        kv: peer.kv_snapshot(),
        ui: peer.ui_snapshot(),
        log: peer.log_lines(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cek_contract::{baseline, ui, Op, ResultMsg, UnknownOpPolicy};

    #[test]
    fn from_wire_profile_and_policy() {
        assert_eq!(
            ApplyProfileKind::from_wire(None),
            ApplyProfileKind::Baseline
        );
        assert_eq!(
            ApplyProfileKind::from_wire(Some("baseline")),
            ApplyProfileKind::Baseline
        );
        assert_eq!(
            ApplyProfileKind::from_wire(Some("ui")),
            ApplyProfileKind::Ui
        );
        assert_eq!(unknown_op_policy_from_wire(None), UnknownOpPolicy::Skip);
        assert_eq!(
            unknown_op_policy_from_wire(Some("skip")),
            UnknownOpPolicy::Skip
        );
        assert_eq!(
            unknown_op_policy_from_wire(Some("fail_batch")),
            UnknownOpPolicy::FailBatch
        );
    }

    #[test]
    fn apply_world_kv_set_matches_peer_apply() {
        let result = ResultMsg::ok(vec![baseline::kv_set("a", serde_json::json!(1))]);
        let world = apply_world(&result, ApplyProfileKind::Baseline, UnknownOpPolicy::Skip);
        let peer = Peer::with_policy(UnknownOpPolicy::Skip);
        let receipt = peer.apply(&result).unwrap();
        assert_eq!(world.receipt, receipt);
        assert_eq!(world.kv.get("a"), Some(&serde_json::json!(1)));
        assert_eq!(world.kv, peer.kv_snapshot());
        assert!(world.ui.is_empty());
        assert!(world.log.is_empty());
    }

    #[test]
    fn apply_world_refuse_is_noop() {
        let world = apply_world(
            &ResultMsg::authority_refusal("no"),
            ApplyProfileKind::Baseline,
            UnknownOpPolicy::Skip,
        );
        assert!(world.kv.is_empty());
        assert!(world.receipt.landed.is_empty());
        assert!(world.receipt.failed.is_empty());
    }

    #[test]
    fn apply_world_ui_profile_lands_morph() {
        let morph = ui::ui_morph("hdr", serde_json::json!({"t": "new"}), None);
        let world = apply_world(
            &ResultMsg::ok(vec![morph]),
            ApplyProfileKind::Ui,
            UnknownOpPolicy::FailBatch,
        );
        assert_eq!(world.receipt.landed.len(), 1);
        assert_eq!(world.ui.get("hdr"), Some(&serde_json::json!({"t": "new"})));
    }

    #[test]
    fn apply_world_fail_batch_aborts_rest() {
        let result = ResultMsg::ok(vec![
            Op {
                ns: "ui.dom".into(),
                name: "morph".into(),
                payload: serde_json::json!({}),
            },
            baseline::kv_set("a", serde_json::json!(1)),
        ]);
        let world = apply_world(
            &result,
            ApplyProfileKind::Baseline,
            UnknownOpPolicy::FailBatch,
        );
        assert_eq!(world.receipt.failed.len(), 2);
        assert!(world.receipt.landed.is_empty());
        assert!(world.kv.is_empty());
    }

    #[test]
    fn apply_world_ui_fail_batch_honors_unknown_op_policy() {
        let result = ResultMsg::ok(vec![
            Op {
                ns: "nope".into(),
                name: "x".into(),
                payload: serde_json::json!({}),
            },
            baseline::kv_set("a", serde_json::json!(1)),
        ]);
        let world = apply_world(&result, ApplyProfileKind::Ui, UnknownOpPolicy::FailBatch);
        assert_eq!(world.receipt.failed.len(), 2);
        assert!(world.receipt.landed.is_empty());
        assert!(world.kv.is_empty());
    }

    #[test]
    fn apply_world_does_not_coerce_none_to_empty_receipt() {
        let src = include_str!("apply.rs");
        let prod = src.split("#[cfg(test)]").next().expect("prod");
        assert!(
            !prod.contains("unwrap_or(Receipt"),
            "apply_world must not map Peer::apply None to a success-shaped empty receipt"
        );
    }
}
