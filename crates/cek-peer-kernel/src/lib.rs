//! # cek-peer-kernel
//!
//! Peer **only applies** Ops. There is no `mint` API in this crate.
//!
//! ## Aging design
//!
//! - Apply is ordered; unknown Ops follow profile policy.
//! - Receipts report landed vs failed — never authority.
//! - Drivers live in `cek-ops-baseline` and `cek-ops-ui`.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use cek_contract::{
    baseline, ui, Manifest, Op, Profile, Receipt, ResultKind, ResultMsg, UnknownOpPolicy,
    LAW_GENERATION, PROFILE_BASELINE, PROFILE_UI,
};
use cek_ops_baseline::KvStore;
use cek_ops_ui::UiStore;
use std::sync::Mutex;

/// Peer kernel with in-memory Baseline (and optional UI) drivers.
pub struct Peer {
    profile: Profile,
    kv: Mutex<KvStore>,
    log: Mutex<Vec<String>>,
    ui: Mutex<UiStore>,
}

impl Default for Peer {
    fn default() -> Self {
        Self::baseline()
    }
}

impl Peer {
    /// Baseline profile Peer (unknown Ops: skip). No UI apply-set.
    pub fn baseline() -> Self {
        Self::with_policy(UnknownOpPolicy::Skip)
    }

    /// Baseline apply-set with an explicit unknown-Op policy.
    pub fn with_policy(unknown_op_policy: UnknownOpPolicy) -> Self {
        Self {
            profile: Profile {
                name: PROFILE_BASELINE.into(),
                apply_set: baseline::BASELINE_OPS
                    .iter()
                    .map(|s| (*s).to_string())
                    .collect(),
                unknown_op_policy,
            },
            kv: Mutex::new(KvStore::new()),
            log: Mutex::new(Vec::new()),
            ui: Mutex::new(UiStore::new()),
        }
    }

    /// Baseline + `ui.dom.*` apply-set (Stage C domain pack).
    pub fn with_ui() -> Self {
        let mut apply: Vec<String> = baseline::BASELINE_OPS
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        apply.extend(ui::UI_OPS.iter().map(|s| (*s).to_string()));
        Self {
            profile: Profile {
                name: PROFILE_UI.into(),
                apply_set: apply,
                unknown_op_policy: UnknownOpPolicy::Skip,
            },
            kv: Mutex::new(KvStore::new()),
            log: Mutex::new(Vec::new()),
            ui: Mutex::new(UiStore::new()),
        }
    }

    /// Declared profile.
    pub fn profile(&self) -> &Profile {
        &self.profile
    }

    /// Manifest for handshake.
    pub fn manifest(&self) -> Manifest {
        Manifest {
            law_generation: LAW_GENERATION.into(),
            accepted_generations: vec![LAW_GENERATION.into()],
            profiles: vec![self.profile.name.clone()],
            fail_closed: Default::default(),
        }
    }

    /// Apply Result Ops in order. Authority refusals are no-ops.
    pub fn apply(&self, result: &ResultMsg) -> Option<Receipt> {
        if matches!(
            result.kind,
            ResultKind::AuthorityRefusal | ResultKind::DispatchError
        ) {
            return Some(Receipt {
                landed: Vec::new(),
                failed: Vec::new(),
            });
        }
        let mut landed = Vec::new();
        let mut failed = Vec::new();
        let mut abort_rest = false;
        for op in &result.ops {
            if abort_rest {
                failed.push(op.clone());
                continue;
            }
            if !self.can_apply(op) {
                match self.profile.unknown_op_policy {
                    UnknownOpPolicy::Skip => {
                        failed.push(op.clone());
                        continue;
                    }
                    UnknownOpPolicy::FailBatch => {
                        failed.push(op.clone());
                        abort_rest = true;
                        continue;
                    }
                }
            }
            match self.apply_one(op) {
                Ok(()) => landed.push(op.clone()),
                Err(()) => {
                    failed.push(op.clone());
                }
            }
        }
        Some(Receipt { landed, failed })
    }

    fn can_apply(&self, op: &Op) -> bool {
        let fq = op.fq();
        self.profile.apply_set.iter().any(|s| s == &fq)
    }

    fn apply_one(&self, op: &Op) -> Result<(), ()> {
        match (op.ns.as_str(), op.name.as_str()) {
            ("kv", "set") => {
                let key = op.payload.get("key").and_then(|v| v.as_str()).ok_or(())?;
                let value = op
                    .payload
                    .get("value")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
                self.kv.lock().map_err(|_| ())?.set(key, value);
                Ok(())
            }
            ("kv", "delete") => {
                let key = op.payload.get("key").and_then(|v| v.as_str()).ok_or(())?;
                self.kv.lock().map_err(|_| ())?.delete(key);
                Ok(())
            }
            ("log", "append") => {
                let msg = op
                    .payload
                    .get("message")
                    .and_then(|v| v.as_str())
                    .ok_or(())?;
                self.log.lock().map_err(|_| ())?.push(msg.to_string());
                Ok(())
            }
            ("ui.dom", "morph") => {
                let target = op
                    .payload
                    .get("target")
                    .and_then(|v| v.as_str())
                    .ok_or(())?;
                let patch = op.payload.get("patch").cloned().ok_or(())?;
                self.ui.lock().map_err(|_| ())?.morph(target, patch);
                Ok(())
            }
            ("ui.dom", "restore") => {
                let target = op
                    .payload
                    .get("target")
                    .and_then(|v| v.as_str())
                    .ok_or(())?;
                let snapshot = op.payload.get("snapshot").cloned().ok_or(())?;
                self.ui.lock().map_err(|_| ())?.restore(target, snapshot);
                Ok(())
            }
            _ => Err(()),
        }
    }

    /// Read kv for tests/demo.
    pub fn kv_get(&self, key: &str) -> Option<serde_json::Value> {
        self.kv.lock().ok()?.get(key)
    }

    /// Log lines for tests/demo.
    pub fn log_lines(&self) -> Vec<String> {
        self.log.lock().map(|g| g.clone()).unwrap_or_default()
    }

    /// UI target for tests/demo.
    pub fn ui_get(&self, target: &str) -> Option<serde_json::Value> {
        self.ui.lock().ok()?.get(target)
    }

    /// Full kv snapshot (ports / WASM).
    pub fn kv_snapshot(&self) -> std::collections::BTreeMap<String, serde_json::Value> {
        self.kv.lock().map(|g| g.snapshot()).unwrap_or_default()
    }

    /// Full UI snapshot (ports / WASM).
    pub fn ui_snapshot(&self) -> std::collections::BTreeMap<String, serde_json::Value> {
        self.ui.lock().map(|g| g.snapshot()).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cek_contract::baseline;

    #[test]
    fn apply_kv_set() {
        let peer = Peer::baseline();
        let result = ResultMsg::ok(vec![baseline::kv_set("a", serde_json::json!(1))]);
        let receipt = peer.apply(&result).unwrap();
        assert_eq!(receipt.landed.len(), 1);
        assert_eq!(peer.kv_get("a"), Some(serde_json::json!(1)));
    }

    #[test]
    fn authority_refusal_no_mutate() {
        let peer = Peer::baseline();
        let result = ResultMsg::authority_refusal("no");
        let _ = peer.apply(&result);
        assert!(peer.kv_get("a").is_none());
    }

    #[test]
    fn unknown_op_skip_continues() {
        let peer = Peer::with_policy(UnknownOpPolicy::Skip);
        let result = ResultMsg::ok(vec![
            Op {
                ns: "ui.dom".into(),
                name: "morph".into(),
                payload: serde_json::json!({}),
            },
            baseline::kv_set("a", serde_json::json!(1)),
        ]);
        let receipt = peer.apply(&result).unwrap();
        assert_eq!(receipt.failed.len(), 1);
        assert_eq!(receipt.landed.len(), 1);
        assert_eq!(peer.kv_get("a"), Some(serde_json::json!(1)));
    }

    #[test]
    fn unknown_op_fail_batch_aborts_rest() {
        let peer = Peer::with_policy(UnknownOpPolicy::FailBatch);
        let result = ResultMsg::ok(vec![
            Op {
                ns: "ui.dom".into(),
                name: "morph".into(),
                payload: serde_json::json!({}),
            },
            baseline::kv_set("a", serde_json::json!(1)),
        ]);
        let receipt = peer.apply(&result).unwrap();
        assert_eq!(receipt.failed.len(), 2);
        assert!(receipt.landed.is_empty());
        assert!(peer.kv_get("a").is_none());
    }

    #[test]
    fn dispatch_error_no_mutate() {
        let peer = Peer::baseline();
        let result = ResultMsg::dispatch_error("miss");
        let rec = peer.apply(&result).unwrap();
        assert!(rec.landed.is_empty());
        assert!(peer.kv_get("a").is_none());
    }

    #[test]
    fn apply_delete_and_log() {
        let peer = Peer::default();
        let _ = peer.apply(&ResultMsg::ok(vec![
            baseline::kv_set("a", serde_json::json!(1)),
            baseline::log_append("hi"),
        ]));
        assert_eq!(peer.kv_get("a"), Some(serde_json::json!(1)));
        assert_eq!(peer.log_lines(), vec!["hi".to_string()]);
        let _ = peer.apply(&ResultMsg::ok(vec![baseline::kv_delete("a")]));
        assert!(peer.kv_get("a").is_none());
        assert_eq!(peer.profile().name, "baseline");
        assert_eq!(peer.manifest().law_generation, LAW_GENERATION);
    }

    #[test]
    fn malformed_payload_is_failed_not_landed() {
        let peer = Peer::baseline();
        let result = ResultMsg::ok(vec![
            Op {
                ns: "kv".into(),
                name: "set".into(),
                payload: serde_json::json!({ "value": 1 }),
            },
            baseline::kv_set("ok", serde_json::json!(2)),
        ]);
        let rec = peer.apply(&result).unwrap();
        assert_eq!(rec.failed.len(), 1);
        assert_eq!(rec.landed.len(), 1);
        assert_eq!(peer.kv_get("ok"), Some(serde_json::json!(2)));
    }

    #[test]
    fn prop_refuse_never_mutates_world() {
        let peer = Peer::baseline();
        let _ = peer.apply(&ResultMsg::ok(vec![baseline::kv_set(
            "seed",
            serde_json::json!(1),
        )]));
        for msg in ["", "no", "action mismatch"] {
            let _ = peer.apply(&ResultMsg::authority_refusal(msg));
            let _ = peer.apply(&ResultMsg::dispatch_error(msg));
        }
        assert_eq!(peer.kv_get("seed"), Some(serde_json::json!(1)));
    }

    #[test]
    fn ui_morph_and_restore() {
        let peer = Peer::with_ui();
        assert!(peer.profile().apply_set.iter().any(|s| s == "ui.dom.morph"));
        let morph = ui::ui_morph(
            "hdr",
            serde_json::json!({"t": "new"}),
            Some(serde_json::json!({"t": "old"})),
        );
        let rec = peer.apply(&ResultMsg::ok(vec![morph.clone()])).unwrap();
        assert_eq!(rec.landed.len(), 1);
        assert_eq!(peer.ui_get("hdr"), Some(serde_json::json!({"t": "new"})));
        let restore = ui::inverse_ui(&morph).unwrap();
        let rec = peer.apply(&ResultMsg::ok(vec![restore])).unwrap();
        assert_eq!(rec.landed.len(), 1);
        assert_eq!(peer.ui_get("hdr"), Some(serde_json::json!({"t": "old"})));
    }

    #[test]
    fn baseline_peer_skips_ui_morph() {
        let peer = Peer::baseline();
        let morph = ui::ui_morph("hdr", serde_json::json!({"t": 1}), None);
        let rec = peer.apply(&ResultMsg::ok(vec![morph])).unwrap();
        assert_eq!(rec.failed.len(), 1);
        assert!(peer.ui_get("hdr").is_none());
    }
}
