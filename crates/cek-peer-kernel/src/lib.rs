//! # cek-peer-kernel
//!
//! Peer **only applies** Ops. There is no `mint` API in this crate.
//!
//! ## Aging design
//!
//! - Apply is ordered; unknown Ops follow profile policy.
//! - Receipts report landed vs failed — never authority.
//! - Drivers live in `cek-ops-baseline` (and future domain crates).

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use cek_contract::{
    baseline, Manifest, Op, Profile, Receipt, ResultKind, ResultMsg, UnknownOpPolicy,
    LAW_GENERATION,
};
use cek_ops_baseline::KvStore;
use std::sync::Mutex;

/// Peer kernel with in-memory Baseline drivers.
pub struct Peer {
    profile: Profile,
    kv: Mutex<KvStore>,
    log: Mutex<Vec<String>>,
}

impl Default for Peer {
    fn default() -> Self {
        Self::baseline()
    }
}

impl Peer {
    /// Baseline profile Peer (unknown Ops: skip).
    pub fn baseline() -> Self {
        Self::with_policy(UnknownOpPolicy::Skip)
    }

    /// Baseline apply-set with an explicit unknown-Op policy.
    pub fn with_policy(unknown_op_policy: UnknownOpPolicy) -> Self {
        Self {
            profile: Profile {
                name: "baseline".into(),
                apply_set: baseline::BASELINE_OPS
                    .iter()
                    .map(|s| (*s).to_string())
                    .collect(),
                unknown_op_policy,
            },
            kv: Mutex::new(KvStore::new()),
            log: Mutex::new(Vec::new()),
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
            profiles: vec![self.profile.name.clone()],
            fail_closed: Default::default(),
        }
    }

    /// Apply Result Ops in order. Authority refusals are no-ops.
    pub fn apply(&self, result: &ResultMsg) -> Option<Receipt> {
        // Refuse and dispatch_error carry no intended effects in this reference.
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
                    // partial apply: continue; reverse uses landed set
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
                ns: "ui".into(),
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
                ns: "ui".into(),
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
}
