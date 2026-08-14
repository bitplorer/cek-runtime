//! Host kernel orchestrator — mature reference implementation.

use crate::{
    BoundAsk, HostError, HostResult, IdemBackend, IdemStore, LineageBackend, LineageStore,
    OnceBackend, OnceStore, ReverseOutcome,
};
use cek_contract::{
    baseline, ops_digest, result_digest, sealed_args_digest, Cap, Intent, Manifest, Op, Receipt,
    ResultMsg, ReverseClass, LAW_GENERATION, PROFILE_BASELINE, PROFILE_PRODUCTION_V1,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::Arc;

/// Reference Host kernel. Stores are trait objects so durable backends can be swapped.
pub struct Host {
    once: Arc<dyn OnceBackend>,
    idem: Arc<dyn IdemBackend>,
    lineage: Arc<dyn LineageBackend>,
    clock: Box<dyn Fn() -> u64 + Send + Sync>,
    /// When true, sealed_args_bind mismatch refuses (always on for maturity).
    enforce_sealed: bool,
}

impl Default for Host {
    fn default() -> Self {
        Self::new()
    }
}

impl Host {
    /// Host with in-memory stores and system-time clock (seconds). Sealed-args enforced.
    pub fn new() -> Self {
        Self::with_backends(
            Arc::new(OnceStore::new()),
            Arc::new(IdemStore::new()),
            Arc::new(LineageStore::new()),
        )
    }

    /// Host with caller-supplied backends and system-time clock.
    pub fn with_backends(
        once: Arc<dyn OnceBackend>,
        idem: Arc<dyn IdemBackend>,
        lineage: Arc<dyn LineageBackend>,
    ) -> Self {
        Self {
            once,
            idem,
            lineage,
            clock: Box::new(|| {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0)
            }),
            enforce_sealed: true,
        }
    }

    /// Host with in-memory stores and a fixed clock (tests / vectors).
    pub fn with_clock(now: u64) -> Self {
        Self::with_stores(
            Arc::new(OnceStore::new()),
            Arc::new(IdemStore::new()),
            Arc::new(LineageStore::new()),
            now,
        )
    }

    /// Host with caller-supplied backends and a fixed clock.
    pub fn with_stores(
        once: Arc<dyn OnceBackend>,
        idem: Arc<dyn IdemBackend>,
        lineage: Arc<dyn LineageBackend>,
        now: u64,
    ) -> Self {
        Self {
            once,
            idem,
            lineage,
            clock: Box::new(move || now),
            enforce_sealed: true,
        }
    }

    /// Shared once backend.
    pub fn once_store(&self) -> Arc<dyn OnceBackend> {
        Arc::clone(&self.once)
    }

    /// Shared lineage backend.
    pub fn lineage_store(&self) -> Arc<dyn LineageBackend> {
        Arc::clone(&self.lineage)
    }

    /// Shared idempotency backend.
    pub fn idem_store(&self) -> Arc<dyn IdemBackend> {
        Arc::clone(&self.idem)
    }

    /// Process manifest.
    pub fn manifest(&self) -> Manifest {
        Manifest {
            law_generation: LAW_GENERATION.into(),
            profiles: vec![PROFILE_BASELINE.into(), PROFILE_PRODUCTION_V1.into()],
            fail_closed: Default::default(),
        }
    }

    /// Mint a Cap (Host bootstrap / policy path).
    pub fn mint(
        &self,
        id: impl Into<String>,
        action: impl Into<String>,
        once: bool,
        not_after: Option<u64>,
    ) -> Cap {
        Cap {
            id: id.into(),
            action: action.into(),
            sealed_args_bind: None,
            not_after,
            once,
            subject: None,
            scopes: Vec::new(),
        }
    }

    /// Mint with sealed-args bind (digest of sealed map).
    pub fn mint_sealed(
        &self,
        id: impl Into<String>,
        action: impl Into<String>,
        once: bool,
        not_after: Option<u64>,
        sealed: &BTreeMap<String, Value>,
    ) -> Cap {
        let mut cap = self.mint(id, action, once, not_after);
        cap.sealed_args_bind = Some(sealed_args_digest(sealed));
        cap
    }

    /// Full submit pipeline: verify → once → project → lineage → Result+digest.
    ///
    /// Cap refusal returns [`ResultMsg`] with `authority_refusal` and **zero** Ops.
    pub fn submit(&self, intent: Intent) -> ResultMsg {
        let now = (self.clock)();
        if let Err(e) = self.verify_cap(&intent, now) {
            return Self::err_result(e);
        }
        // Idempotency is checked after Cap verify and **before** once-ensure
        // so a retry of a once-Cap returns the cached Result instead of refusing.
        if let Some(ref key) = intent.idempotency_key {
            match self.idempotency_lookup(key, &intent) {
                Ok(Some(prior)) => return prior,
                Ok(None) => {}
                Err(e) => return Self::err_result(e),
            }
        }
        match self.once.ensure_available(&intent.cap.id, intent.cap.once) {
            Ok(()) => self.dispatch_and_finish(BoundAsk { intent, now }),
            Err(e) => Self::err_result(e),
        }
    }

    fn err_result(e: HostError) -> ResultMsg {
        match e {
            HostError::Authority(msg) => {
                let mut r = ResultMsg::authority_refusal(msg);
                r.digest = Some(result_digest("authority_refusal", &[], r.error.as_deref()));
                r
            }
            HostError::OnceStoreDown => {
                let mut r = ResultMsg::authority_refusal("once store unavailable");
                r.digest = Some(result_digest("authority_refusal", &[], r.error.as_deref()));
                r
            }
            HostError::IdemStoreDown => {
                let mut r = ResultMsg::authority_refusal("idempotency store unavailable");
                r.digest = Some(result_digest("authority_refusal", &[], r.error.as_deref()));
                r
            }
            e => {
                let mut r = ResultMsg::dispatch_error(e.to_string());
                r.digest = Some(result_digest("dispatch_error", &[], r.error.as_deref()));
                r
            }
        }
    }

    /// Cap integrity only (action, expiry, sealed-args, non-empty). No once, no idem.
    fn verify_cap(&self, intent: &Intent, now: u64) -> HostResult<()> {
        let cap = &intent.cap;
        if intent.action != cap.action {
            return Err(HostError::Authority(format!(
                "action mismatch: intent `{}` vs Cap `{}`",
                intent.action, cap.action
            )));
        }
        if let Some(na) = cap.not_after {
            if now >= na {
                return Err(HostError::Authority(format!(
                    "Cap expired: now={now} not_after={na}"
                )));
            }
        }
        if self.enforce_sealed {
            if let Some(ref bind) = cap.sealed_args_bind {
                let got = sealed_args_digest(&intent.args);
                if &got != bind {
                    return Err(HostError::Authority(format!(
                        "sealed-args bind mismatch: cap expects {bind}, got {got}"
                    )));
                }
            }
        }
        if intent.action.is_empty() || cap.action.is_empty() {
            return Err(HostError::Authority("empty action is not allowed".into()));
        }
        if cap.id.is_empty() {
            return Err(HostError::Authority("empty Cap id is not allowed".into()));
        }
        Ok(())
    }

    /// Same key + same projected digest → cached Result.
    /// Same key + different digest → authority refusal.
    /// Missing key → None (first use).
    fn idempotency_lookup(&self, key: &str, intent: &Intent) -> HostResult<Option<ResultMsg>> {
        let Some(prior) = self.idem.get(key)? else {
            return Ok(None);
        };
        match project_ops(intent) {
            Ok(ops) => {
                let digest = result_digest("ok", &ops, None);
                if prior.digest.as_deref() == Some(digest.as_str()) {
                    Ok(Some(prior))
                } else {
                    Err(HostError::Authority(format!(
                        "idempotency conflict for key `{key}`"
                    )))
                }
            }
            Err(_) => Err(HostError::Authority(format!(
                "idempotency conflict for key `{key}`"
            ))),
        }
    }

    /// Stage 1–2: verify Cap + sealed + once → BoundAsk.
    ///
    /// Callers that already handled idempotency (or have none) use this.
    /// [`Host::submit`] checks idempotency **before** once-ensure.
    pub fn verify_and_bind(&self, intent: Intent, now: u64) -> HostResult<BoundAsk> {
        self.verify_cap(&intent, now)?;
        self.once
            .ensure_available(&intent.cap.id, intent.cap.once)?;
        Ok(BoundAsk { intent, now })
    }

    fn dispatch_and_finish(&self, bound: BoundAsk) -> ResultMsg {
        let intent = bound.intent();

        match project_ops(intent) {
            Ok(ops) => {
                let kind = "ok";
                let digest = result_digest(kind, &ops, None);
                let mut r = ResultMsg::ok(ops.clone());
                r.digest = Some(digest.clone());

                // Idempotency: record or detect conflict (after we know digest)
                if let Some(ref key) = intent.idempotency_key {
                    match self.idem.put_or_check(key, &digest, &r) {
                        Ok(crate::IdemOutcome::Recorded) => {}
                        Ok(crate::IdemOutcome::ReplaySame { result }) => {
                            return result;
                        }
                        Err(HostError::Authority(msg)) => {
                            let mut rr = ResultMsg::authority_refusal(msg);
                            rr.digest =
                                Some(result_digest("authority_refusal", &[], rr.error.as_deref()));
                            return rr;
                        }
                        Err(e) => {
                            let mut rr = ResultMsg::dispatch_error(e.to_string());
                            rr.digest =
                                Some(result_digest("dispatch_error", &[], rr.error.as_deref()));
                            return rr;
                        }
                    }
                }

                // Commit once-Cap only after successful project (no burn on dispatch miss).
                if let Err(e) = self.once.commit(&intent.cap.id, intent.cap.once) {
                    let mut rr = ResultMsg::authority_refusal(e.to_string());
                    rr.digest = Some(result_digest("authority_refusal", &[], rr.error.as_deref()));
                    return rr;
                }

                if let Some(ref aid) = intent.activity_id {
                    if aid.is_empty() {
                        let mut rr = ResultMsg::dispatch_error("empty activity_id");
                        rr.digest = Some(result_digest("dispatch_error", &[], rr.error.as_deref()));
                        return rr;
                    }
                    let inverse = inverse_for(&ops);
                    let class = if inverse.is_empty() {
                        ReverseClass::NonReversible
                    } else {
                        ReverseClass::Inverse
                    };
                    if let Err(e) = self.lineage.commit(
                        &intent.cap.id,
                        Some(aid),
                        &intent.action,
                        ops.clone(),
                        class,
                        inverse,
                    ) {
                        let mut rr = ResultMsg::dispatch_error(e.to_string());
                        rr.digest = Some(result_digest("dispatch_error", &[], rr.error.as_deref()));
                        return rr;
                    }
                }

                let _ = ops_digest(&r.ops);
                r
            }
            Err(msg) => {
                // Dispatch failure: once-Cap NOT committed — Cap remains usable.
                let mut r = ResultMsg::dispatch_error(msg);
                r.digest = Some(result_digest("dispatch_error", &[], r.error.as_deref()));
                r
            }
        }
    }

    /// Record Peer receipt against latest lineage for Activity (landed-first reverse).
    pub fn report_receipt(&self, activity_id: &str, receipt: &Receipt) -> HostResult<()> {
        self.lineage
            .annotate_landed_latest_for_activity(activity_id, receipt.landed.clone())
    }

    /// End Activity → reverse lineage.
    ///
    /// Preference: if landed_ops annotated → build inverse from landed;
    /// else use inverse_ops recorded at commit; NonReversible listed honestly.
    pub fn end_activity(&self, activity_id: &str) -> HostResult<ReverseOutcome> {
        if activity_id.is_empty() {
            return Err(HostError::Lineage("empty activity_id".into()));
        }
        self.lineage.mark_ended(activity_id)?;
        let entries = self.lineage.for_activity(activity_id)?;
        let mut ops = Vec::new();
        let mut non_reversible = Vec::new();
        let mut used_landed = false;
        for entry in entries.into_iter().rev() {
            match entry.reverse_class {
                ReverseClass::Inverse => {
                    if !entry.landed_ops.is_empty() {
                        used_landed = true;
                        ops.extend(inverse_for(&entry.landed_ops));
                    } else {
                        ops.extend(entry.inverse_ops);
                    }
                }
                ReverseClass::Compensation => {
                    non_reversible.push(entry.id);
                }
                ReverseClass::NonReversible => {
                    non_reversible.push(entry.id);
                }
            }
        }
        Ok(ReverseOutcome {
            ops,
            non_reversible,
            used_landed,
        })
    }
}

fn project_ops(intent: &Intent) -> Result<Vec<Op>, String> {
    match intent.action.as_str() {
        "kv.write" => {
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
        "kv.delete" => {
            let key = intent
                .args
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "kv.delete requires string args.key".to_string())?;
            if key.is_empty() {
                return Err("kv.delete key must be non-empty".into());
            }
            Ok(vec![baseline::kv_delete(key)])
        }
        "log.append" => {
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

fn inverse_for(ops: &[Op]) -> Vec<Op> {
    let mut inv = Vec::new();
    for op in ops.iter().rev() {
        if op.ns == "kv" && op.name == "set" {
            if let Some(key) = op.payload.get("key").and_then(|v| v.as_str()) {
                inv.push(baseline::kv_delete(key));
            }
        }
        // kv.delete / log.append: non-reversible without snapshot
    }
    inv
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FileIdemStore, FileLineageStore, FileOnceStore};
    use cek_contract::ResultKind;
    use serde_json::json;
    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn intent_write(cap: Cap, key: &str, val: Value) -> Intent {
        let mut args = BTreeMap::new();
        args.insert("key".into(), json!(key));
        args.insert("value".into(), val);
        Intent {
            action: "kv.write".into(),
            args,
            cap,
            trace: None,
            idempotency_key: None,
            activity_id: Some("act-1".into()),
        }
    }

    #[test]
    fn refuse_action_mismatch_zero_ops() {
        let host = Host::with_clock(1000);
        let cap = host.mint("c1", "kv.read", false, None);
        let r = host.submit(intent_write(cap, "a", json!(1)));
        assert!(matches!(r.kind, ResultKind::AuthorityRefusal));
        assert!(r.ops.is_empty());
        assert!(r.digest.is_some());
    }

    #[test]
    fn refuse_expired() {
        let host = Host::with_clock(2000);
        let cap = host.mint("c2", "kv.write", false, Some(1500));
        let r = host.submit(intent_write(cap, "a", json!(1)));
        assert!(matches!(r.kind, ResultKind::AuthorityRefusal));
        assert!(r.ops.is_empty());
    }

    #[test]
    fn once_second_use_refuses() {
        let host = Host::with_clock(1000);
        let cap = host.mint("once-1", "kv.write", true, None);
        let r1 = host.submit(intent_write(cap.clone(), "a", json!(1)));
        assert!(matches!(r1.kind, ResultKind::Ok));
        let r2 = host.submit(intent_write(cap, "a", json!(2)));
        assert!(matches!(r2.kind, ResultKind::AuthorityRefusal));
        assert!(r2.ops.is_empty());
    }

    #[test]
    fn baseline_kv_set_ok_with_digest() {
        let host = Host::with_clock(1000);
        let cap = host.mint("c3", "kv.write", false, None);
        let r = host.submit(intent_write(cap, "greeting", json!("hello")));
        assert!(matches!(r.kind, ResultKind::Ok));
        assert_eq!(r.ops.len(), 1);
        assert!(r.digest.as_ref().unwrap().starts_with("cek1:"));
    }

    #[test]
    fn sealed_args_mismatch_refuses() {
        let host = Host::with_clock(1000);
        let mut sealed = BTreeMap::new();
        sealed.insert("key".into(), json!("fixed"));
        sealed.insert("value".into(), json!(1));
        let cap = host.mint_sealed("c-seal", "kv.write", false, None, &sealed);
        let mut args = BTreeMap::new();
        args.insert("key".into(), json!("fixed"));
        args.insert("value".into(), json!(999)); // tamper
        let intent = Intent {
            action: "kv.write".into(),
            args,
            cap,
            trace: None,
            idempotency_key: None,
            activity_id: None,
        };
        let r = host.submit(intent);
        assert!(matches!(r.kind, ResultKind::AuthorityRefusal));
        assert!(r.ops.is_empty());
    }

    #[test]
    fn sealed_args_match_ok() {
        let host = Host::with_clock(1000);
        let mut sealed = BTreeMap::new();
        sealed.insert("key".into(), json!("fixed"));
        sealed.insert("value".into(), json!(1));
        let cap = host.mint_sealed("c-seal2", "kv.write", false, None, &sealed);
        let intent = Intent {
            action: "kv.write".into(),
            args: sealed,
            cap,
            trace: None,
            idempotency_key: None,
            activity_id: None,
        };
        let r = host.submit(intent);
        assert!(matches!(r.kind, ResultKind::Ok));
    }

    #[test]
    fn end_activity_emits_inverse_delete() {
        let host = Host::with_clock(1000);
        let cap = host.mint("c4", "kv.write", false, None);
        let r = host.submit(intent_write(cap, "k", json!(1)));
        assert!(matches!(r.kind, ResultKind::Ok));
        let rev = host.end_activity("act-1").unwrap();
        assert_eq!(rev.ops.len(), 1);
        assert_eq!(rev.ops[0].fq(), "kv.delete");
    }

    #[test]
    fn receipt_annotates_landed_reverse() {
        let host = Host::with_clock(1000);
        let cap = host.mint("c5", "kv.write", false, None);
        let r = host.submit(intent_write(cap, "k", json!(1)));
        assert!(matches!(r.kind, ResultKind::Ok));
        let receipt = Receipt {
            landed: r.ops.clone(),
            failed: vec![],
        };
        host.report_receipt("act-1", &receipt).unwrap();
        let rev = host.end_activity("act-1").unwrap();
        assert!(rev.used_landed);
        assert_eq!(rev.ops[0].fq(), "kv.delete");
    }

    #[test]
    fn once_not_burned_on_dispatch_error() {
        let host = Host::with_clock(1000);
        let cap = host.mint("once-dispatch", "no.such.action", true, None);
        let intent = Intent {
            action: "no.such.action".into(),
            args: BTreeMap::new(),
            cap: cap.clone(),
            trace: None,
            idempotency_key: None,
            activity_id: None,
        };
        let r = host.submit(intent);
        assert!(matches!(r.kind, ResultKind::DispatchError));
        assert!(!host.once_store().is_consumed("once-dispatch"));
        // Cap still usable for a valid action after policy change would need re-mint;
        // here we only assert burn did not happen.
    }

    #[test]
    fn empty_action_refuses() {
        let host = Host::with_clock(1000);
        let mut cap = host.mint("c-empty", "kv.write", false, None);
        cap.action = String::new();
        let intent = Intent {
            action: String::new(),
            args: BTreeMap::new(),
            cap,
            trace: None,
            idempotency_key: None,
            activity_id: None,
        };
        let r = host.submit(intent);
        assert!(matches!(r.kind, ResultKind::AuthorityRefusal));
        assert!(r.ops.is_empty());
    }

    #[test]
    fn empty_kv_key_dispatch_error() {
        let host = Host::with_clock(1000);
        let cap = host.mint("c-ek", "kv.write", false, None);
        let r = host.submit(intent_write(cap, "", json!(1)));
        assert!(matches!(r.kind, ResultKind::DispatchError));
    }

    #[test]
    fn double_end_activity_errors() {
        let host = Host::with_clock(1000);
        let cap = host.mint("c-de", "kv.write", false, None);
        let _ = host.submit(intent_write(cap, "k", json!(1)));
        assert!(host.end_activity("act-1").is_ok());
        assert!(host.end_activity("act-1").is_err());
    }

    #[test]
    fn commit_after_activity_ended_is_dispatch_error() {
        let host = Host::with_clock(1000);
        let cap = host.mint("c-cae", "kv.write", false, None);
        let r1 = host.submit(intent_write(cap.clone(), "k", json!(1)));
        assert!(matches!(r1.kind, ResultKind::Ok));
        host.end_activity("act-1").unwrap();
        let r2 = host.submit(intent_write(cap, "k", json!(2)));
        assert!(matches!(r2.kind, ResultKind::DispatchError));
        assert!(r2.ops.is_empty());
    }

    #[test]
    fn idempotent_replay_returns_same_digest() {
        let host = Host::with_clock(1000);
        let cap = host.mint("c-idem", "kv.write", false, None);
        let mut i1 = intent_write(cap.clone(), "k", json!(1));
        i1.idempotency_key = Some("same-key".into());
        i1.activity_id = None;
        let r1 = host.submit(i1.clone());
        assert!(matches!(r1.kind, ResultKind::Ok));
        let r2 = host.submit(i1);
        assert!(matches!(r2.kind, ResultKind::Ok));
        assert_eq!(r1.digest, r2.digest);
        assert_eq!(r1.ops, r2.ops);
    }

    #[test]
    fn once_cap_idempotent_retry_returns_cached() {
        let host = Host::with_clock(1000);
        let cap = host.mint("c-once-idem", "kv.write", true, None);
        let mut i1 = intent_write(cap.clone(), "k", json!(1));
        i1.idempotency_key = Some("once-retry".into());
        i1.activity_id = None;
        let r1 = host.submit(i1.clone());
        assert!(matches!(r1.kind, ResultKind::Ok));
        assert!(host.once_store().is_consumed("c-once-idem"));
        let r2 = host.submit(i1);
        assert!(matches!(r2.kind, ResultKind::Ok));
        assert_eq!(r1.digest, r2.digest);
        assert!(r2.ops.is_empty() == false);
    }

    #[test]
    fn idempotency_conflict_refuses() {
        let host = Host::with_clock(1000);
        let cap = host.mint("c-idc", "kv.write", false, None);
        let mut i1 = intent_write(cap.clone(), "k", json!(1));
        i1.idempotency_key = Some("conflict-key".into());
        i1.activity_id = None;
        let r1 = host.submit(i1);
        assert!(matches!(r1.kind, ResultKind::Ok));
        let mut i2 = intent_write(cap, "k", json!(999));
        i2.idempotency_key = Some("conflict-key".into());
        i2.activity_id = None;
        let r2 = host.submit(i2);
        assert!(matches!(r2.kind, ResultKind::AuthorityRefusal));
        assert!(r2.ops.is_empty());
    }

    #[test]
    fn durable_file_host_once_and_reverse_survive_reopen() {
        static N: AtomicU64 = AtomicU64::new(1);
        let dir = std::env::temp_dir().join(format!(
            "cek-host-file-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let cap_id;
        {
            let host = Host::with_stores(
                Arc::new(FileOnceStore::open(&dir).unwrap()),
                Arc::new(FileIdemStore::open(&dir).unwrap()),
                Arc::new(FileLineageStore::open(&dir).unwrap()),
                1000,
            );
            let cap = host.mint("once-file", "kv.write", true, None);
            cap_id = cap.id.clone();
            let r = host.submit(intent_write(cap, "greet", json!("hi")));
            assert!(matches!(r.kind, ResultKind::Ok));
            let receipt = Receipt {
                landed: r.ops.clone(),
                failed: vec![],
            };
            host.report_receipt("act-1", &receipt).unwrap();
        }

        let host2 = Host::with_stores(
            Arc::new(FileOnceStore::open(&dir).unwrap()),
            Arc::new(FileIdemStore::open(&dir).unwrap()),
            Arc::new(FileLineageStore::open(&dir).unwrap()),
            1000,
        );
        assert!(host2.once_store().is_consumed(&cap_id));
        let cap2 = host2.mint("once-file", "kv.write", true, None);
        let r2 = host2.submit(intent_write(cap2, "greet", json!("again")));
        assert!(matches!(r2.kind, ResultKind::AuthorityRefusal));
        assert!(r2.ops.is_empty());
        let rev = host2.end_activity("act-1").unwrap();
        assert!(rev.used_landed);
        assert_eq!(rev.ops[0].fq(), "kv.delete");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn host_new_default_manifest_and_bind() {
        let host = Host::new();
        let _ = Host::default();
        let m = host.manifest();
        assert_eq!(m.law_generation, LAW_GENERATION);
        assert!(m.profiles.contains(&PROFILE_BASELINE.to_string()));
        let cap = host.mint("c-new", "kv.write", false, None);
        let bound = host
            .verify_and_bind(intent_write(cap, "k", json!(1)), 1000)
            .unwrap();
        assert_eq!(bound.now(), 1000);
        assert_eq!(bound.intent().action, "kv.write");
        let _ = host.idem_store();
        let _ = host.lineage_store();
    }

    #[test]
    fn empty_activity_id_is_dispatch_error() {
        let host = Host::with_clock(1000);
        let cap = host.mint("c-ea", "kv.write", false, None);
        let mut i = intent_write(cap, "k", json!(1));
        i.activity_id = Some(String::new());
        let r = host.submit(i);
        assert!(matches!(r.kind, ResultKind::DispatchError));
        assert!(r.ops.is_empty());
    }

    #[test]
    fn missing_kv_key_and_log_message_dispatch() {
        let host = Host::with_clock(1000);
        let cap = host.mint("c-mk", "kv.write", false, None);
        let r = host.submit(Intent {
            action: "kv.write".into(),
            args: BTreeMap::new(),
            cap,
            trace: None,
            idempotency_key: None,
            activity_id: None,
        });
        assert!(matches!(r.kind, ResultKind::DispatchError));
        let cap = host.mint("c-lm", "log.append", false, None);
        let r = host.submit(Intent {
            action: "log.append".into(),
            args: BTreeMap::new(),
            cap,
            trace: None,
            idempotency_key: None,
            activity_id: None,
        });
        assert!(matches!(r.kind, ResultKind::DispatchError));
    }

    #[test]
    fn log_append_activity_is_non_reversible() {
        let host = Host::with_clock(1000);
        let cap = host.mint("c-nr", "log.append", false, None);
        let mut args = BTreeMap::new();
        args.insert("message".into(), json!("hi"));
        let r = host.submit(Intent {
            action: "log.append".into(),
            args,
            cap,
            trace: None,
            idempotency_key: None,
            activity_id: Some("act-log".into()),
        });
        assert!(matches!(r.kind, ResultKind::Ok));
        let rev = host.end_activity("act-log").unwrap();
        assert!(rev.ops.is_empty());
        assert!(!rev.non_reversible.is_empty());
        assert!(!rev.used_landed);
    }

    #[test]
    fn compensation_listed_honestly() {
        let host = Host::with_clock(1000);
        host.lineage_store()
            .commit(
                "cap",
                Some("act-comp"),
                "kv.write",
                vec![baseline::kv_set("k", json!(1))],
                ReverseClass::Compensation,
                vec![],
            )
            .unwrap();
        let rev = host.end_activity("act-comp").unwrap();
        assert!(rev.ops.is_empty());
        assert_eq!(rev.non_reversible.len(), 1);
    }

    #[test]
    fn report_receipt_unknown_activity_errors() {
        let host = Host::with_clock(1000);
        let rec = Receipt {
            landed: vec![],
            failed: vec![],
        };
        assert!(host.report_receipt("no-such", &rec).is_err());
    }

    #[test]
    fn once_try_consume_and_file_stores_helper() {
        let once = OnceStore::new();
        once.try_consume("x", true).unwrap();
        assert!(once.try_consume("x", true).is_err());
        assert!(once.try_consume("y", false).is_ok());

        static N: AtomicU64 = AtomicU64::new(1);
        let dir = std::env::temp_dir().join(format!(
            "cek-filestores-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let stores = crate::FileStores::open(&dir).unwrap();
        stores.once.commit("c", true).unwrap();
        drop(stores);
        let stores2 = crate::FileStores::open(&dir).unwrap();
        assert!(stores2.once.is_consumed("c"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn expiry_at_exact_not_after_refuses() {
        let host = Host::with_clock(1500);
        let cap = host.mint("c-eq", "kv.write", false, Some(1500));
        let r = host.submit(intent_write(cap, "a", json!(1)));
        assert!(matches!(r.kind, ResultKind::AuthorityRefusal));
        assert!(r.ops.is_empty());
    }
}
