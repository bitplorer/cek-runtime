//! Host kernel orchestrator — mature reference implementation.

use crate::project::{dispatch_ops, inverse_ops, project_authorized};
use crate::{
    BoundAsk, HostError, HostResult, IdemBackend, IdemStore, LineageBackend, LineageStore,
    OnceBackend, OnceStore, ReverseOutcome,
};
use cek_contract::{
    ops_digest, result_digest, sealed_args_digest, Cap, Intent, Manifest, Op, Receipt, ResultMsg,
    ReverseClass, LAW_GENERATION, PROFILE_BASELINE, PROFILE_PRODUCTION_V1,
};
use ed25519_dalek::{SigningKey, VerifyingKey};
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Reference Host kernel. Stores are trait objects so durable backends can be swapped.
pub struct Host {
    pub(crate) once: Arc<dyn OnceBackend>,
    pub(crate) idem: Arc<dyn IdemBackend>,
    pub(crate) lineage: Arc<dyn LineageBackend>,
    pub(crate) clock: Box<dyn Fn() -> u64 + Send + Sync>,
    /// When true, sealed_args_bind mismatch refuses (always on for maturity).
    pub(crate) enforce_sealed: bool,
    /// When set, mint attaches HMAC and verify refuses missing/invalid sigs.
    pub(crate) signing_key: Option<[u8; 32]>,
    /// Optional Ed25519 signer (Host mint / attenuate).
    pub(crate) ed_sign: Option<SigningKey>,
    /// Trusted Ed25519 public keys (verify; rotation window).
    pub(crate) ed_trust: Vec<VerifyingKey>,
    /// Law generations this Host accepts (always includes current).
    pub(crate) accepted_generations: Vec<String>,
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
            signing_key: None,
            ed_sign: None,
            ed_trust: Vec::new(),
            accepted_generations: vec![LAW_GENERATION.into()],
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
            signing_key: None,
            ed_sign: None,
            ed_trust: Vec::new(),
            accepted_generations: vec![LAW_GENERATION.into()],
        }
    }

    /// Enable Cap HMAC (Host policy). Mint attaches `sig`; verify refuses otherwise.
    pub fn with_hmac_key(mut self, key: [u8; 32]) -> Self {
        self.signing_key = Some(key);
        self
    }

    /// Enable Ed25519 mint + verify (Host policy). Seed is RFC 8032 secret.
    /// The matching public key is trusted automatically.
    pub fn with_ed25519(mut self, seed: [u8; 32]) -> Self {
        let sk = crate::sign::signing_key(&seed);
        self.ed_trust.push(sk.verifying_key());
        self.ed_sign = Some(sk);
        self
    }

    /// Trust an additional Ed25519 public key (rotation / dual-speak window).
    pub fn trust_ed25519(mut self, public: [u8; 32]) -> HostResult<Self> {
        self.ed_trust.push(crate::sign::verifying_key(&public)?);
        Ok(self)
    }

    /// This Host's Ed25519 public key, if it can mint Ed25519 Caps.
    pub fn ed25519_public(&self) -> Option<[u8; 32]> {
        self.ed_sign
            .as_ref()
            .map(|sk| sk.verifying_key().to_bytes())
    }

    /// Accept an additional law generation (dual-speak window). Current is always accepted.
    pub fn accept_generation(mut self, gen: impl Into<String>) -> HostResult<Self> {
        let g = gen.into();
        if g.trim().is_empty() {
            return Err(HostError::Authority(
                "empty law generation is not allowed".into(),
            ));
        }
        if !self.accepted_generations.contains(&g) {
            self.accepted_generations.push(g);
        }
        Ok(self)
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
            accepted_generations: self.accepted_generations.clone(),
            profiles: vec![PROFILE_BASELINE.into(), PROFILE_PRODUCTION_V1.into()],
            fail_closed: cek_contract::FailClosed {
                cap_signatures: self.requires_cap_sig(),
                ..Default::default()
            },
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
        let cap = Cap {
            id: id.into(),
            action: action.into(),
            sealed_args_bind: None,
            not_after,
            once,
            subject: None,
            scopes: Vec::new(),
            sig: None,
            law_generation: Some(LAW_GENERATION.into()),
        };
        self.attach_sig(cap)
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
        cap.sig = None;
        self.attach_sig(cap)
    }

    /// Derive a narrower Cap. Widening scopes is refused (fail closed).
    pub fn attenuate(
        &self,
        parent: &Cap,
        new_id: impl Into<String>,
        scopes: Vec<String>,
    ) -> HostResult<Cap> {
        if !crate::scope::can_attenuate(&parent.scopes, &scopes) {
            return Err(HostError::Authority(
                "attenuation would widen scopes".into(),
            ));
        }
        let id = new_id.into();
        if id.is_empty() {
            return Err(HostError::Authority("empty Cap id is not allowed".into()));
        }
        let mut cap = parent.clone();
        cap.id = id;
        cap.scopes = scopes;
        cap.sig = None;
        Ok(self.attach_sig(cap))
    }

    /// Attach or refresh Host-policy signature. No-op when this Host has no key.
    /// Ed25519 wins when both schemes are configured (HMAC remains verifiable).
    pub fn attach_sig(&self, cap: Cap) -> Cap {
        if let Some(sk) = &self.ed_sign {
            return crate::sign::attach_ed25519(sk, cap);
        }
        if let Some(key) = &self.signing_key {
            return crate::sign::attach_hmac(key, cap);
        }
        cap
    }

    /// Lower authorized Ops to Baseline (ui.* → kv.set). Does not change submit.
    pub fn lower_ops(ops: &[Op]) -> Vec<Op> {
        ops.iter()
            .filter_map(cek_contract::lower_to_baseline)
            .collect()
    }

    /// Full submit pipeline (LAW §4 / CORE 06 Host duties):
    /// verify Cap → consume once / idempotency bind → dispatch → **record lineage**
    /// → **project Ops** → Result+digest.
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
            if key.is_empty() {
                return Self::err_result(HostError::Authority(
                    "empty idempotency key is not allowed".into(),
                ));
            }
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

    /// Same key + same projected digest → cached Result.
    /// Same key + different digest → authority refusal.
    /// Missing key → None (first use).
    fn idempotency_lookup(&self, key: &str, intent: &Intent) -> HostResult<Option<ResultMsg>> {
        let Some(prior) = self.idem.get(key)? else {
            return Ok(None);
        };
        match dispatch_ops(intent) {
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

    fn law4_note(step: &'static str) {
        #[cfg(test)]
        tests::LAW4_STEPS.with(|s| s.borrow_mut().push(step));
        let _ = step;
    }

    fn dispatch_and_finish(&self, bound: BoundAsk) -> ResultMsg {
        let intent = bound.intent();

        // LAW §4 step 3: Dispatch → authorized Ops. Miss does not burn a once-Cap.
        let authorized = match dispatch_ops(intent) {
            Ok(ops) => ops,
            Err(msg) => {
                let mut r = ResultMsg::dispatch_error(msg);
                r.digest = Some(result_digest("dispatch_error", &[], r.error.as_deref()));
                return r;
            }
        };
        Self::law4_note("dispatch");

        let digest = result_digest("ok", &authorized, None);

        // Idempotency bind after digest is known, **before** lineage (no second cause).
        if let Some(ref key) = intent.idempotency_key {
            let cached = ResultMsg {
                kind: cek_contract::ResultKind::Ok,
                ops: authorized.clone(),
                error: None,
                digest: Some(digest.clone()),
            };
            match self.idem.put_or_check(key, &digest, &cached) {
                Ok(crate::IdemOutcome::Recorded) => {}
                Ok(crate::IdemOutcome::ReplaySame { result }) => {
                    return result;
                }
                Err(HostError::Authority(msg)) => {
                    let mut rr = ResultMsg::authority_refusal(msg);
                    rr.digest = Some(result_digest("authority_refusal", &[], rr.error.as_deref()));
                    return rr;
                }
                Err(e) => {
                    let mut rr = ResultMsg::dispatch_error(e.to_string());
                    rr.digest = Some(result_digest("dispatch_error", &[], rr.error.as_deref()));
                    return rr;
                }
            }
        }

        // Consume once only after successful dispatch (no burn on dispatch miss).
        if let Err(e) = self.once.commit(&intent.cap.id, intent.cap.once) {
            let mut rr = ResultMsg::authority_refusal(e.to_string());
            rr.digest = Some(result_digest("authority_refusal", &[], rr.error.as_deref()));
            return rr;
        }

        // LAW §4 step 4: Record lineage (authorized set + reverse plan) **before** project.
        if let Some(ref aid) = intent.activity_id {
            if aid.is_empty() {
                let mut rr = ResultMsg::dispatch_error("empty activity_id");
                rr.digest = Some(result_digest("dispatch_error", &[], rr.error.as_deref()));
                return rr;
            }
            let inverse = inverse_ops(&authorized);
            let class = if inverse.is_empty() {
                ReverseClass::NonReversible
            } else {
                ReverseClass::Inverse
            };
            if let Err(e) = self.lineage.commit(
                &intent.cap.id,
                Some(aid),
                &intent.action,
                authorized.clone(),
                class,
                inverse,
            ) {
                let mut rr = ResultMsg::dispatch_error(e.to_string());
                rr.digest = Some(result_digest("dispatch_error", &[], rr.error.as_deref()));
                return rr;
            }
            Self::law4_note("record_lineage");
        }

        // LAW §4 steps 5–6: Project Ops onto Result, then return.
        // First-cut project is identity (profile negotiate / Baseline lower is out of scope).
        let projected = project_authorized(authorized);
        Self::law4_note("project");
        let mut r = ResultMsg::ok(projected);
        r.digest = Some(digest);
        let _ = ops_digest(&r.ops);
        r
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
                        ops.extend(inverse_ops(&entry.landed_ops));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FileIdemStore, FileLineageStore, FileOnceStore};
    use cek_contract::{baseline, ResultKind};
    use serde_json::json;
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicU64, Ordering};

    thread_local! {
        pub(super) static LAW4_STEPS: RefCell<Vec<&'static str>> = RefCell::new(Vec::new());
    }

    fn law4_take() -> Vec<&'static str> {
        LAW4_STEPS.with(|s| std::mem::take(&mut *s.borrow_mut()))
    }

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

    fn intent_morph(cap: Cap, target: &str, patch: Value, snapshot: Option<Value>) -> Intent {
        let mut args = BTreeMap::new();
        args.insert("target".into(), json!(target));
        args.insert("patch".into(), patch);
        if let Some(s) = snapshot {
            args.insert("snapshot".into(), s);
        }
        Intent {
            action: "ui.morph".into(),
            args,
            cap,
            trace: None,
            idempotency_key: None,
            activity_id: Some("act-ui".into()),
        }
    }

    #[test]
    fn ui_morph_projects_and_restore_reverse() {
        let host = Host::with_clock(1000);
        let cap = host.mint("c-ui", "ui.morph", false, None);
        let r = host.submit(intent_morph(
            cap,
            "hdr",
            json!({"t": "new"}),
            Some(json!({"t": "old"})),
        ));
        assert!(matches!(r.kind, ResultKind::Ok));
        assert_eq!(r.ops[0].fq(), "ui.dom.morph");
        let rev = host.end_activity("act-ui").unwrap();
        assert_eq!(rev.ops[0].fq(), "ui.dom.restore");
    }

    #[test]
    fn ui_morph_without_snapshot_is_non_reversible() {
        let host = Host::with_clock(1000);
        let cap = host.mint("c-ui2", "ui.morph", false, None);
        let r = host.submit(intent_morph(cap, "hdr", json!({"t": 1}), None));
        assert!(matches!(r.kind, ResultKind::Ok));
        let rev = host.end_activity("act-ui").unwrap();
        assert!(rev.ops.is_empty());
        assert!(!rev.non_reversible.is_empty());
    }

    #[test]
    fn scope_denies_wrong_key() {
        let host = Host::with_clock(1000);
        let mut cap = host.mint("c-sc", "kv.write", false, None);
        cap.scopes = vec!["kv:allowed".into()];
        let r = host.submit(intent_write(cap, "other", json!(1)));
        assert!(matches!(r.kind, ResultKind::AuthorityRefusal));
        assert!(r.ops.is_empty());
    }

    #[test]
    fn scope_allows_matching_key() {
        let host = Host::with_clock(1000);
        let mut cap = host.mint("c-sc2", "kv.write", false, None);
        cap.scopes = vec!["kv:greeting".into()];
        let r = host.submit(intent_write(cap, "greeting", json!(1)));
        assert!(matches!(r.kind, ResultKind::Ok));
    }

    #[test]
    fn attenuate_narrows_and_refuses_widen() {
        let host = Host::with_clock(1000);
        let mut parent = host.mint("p", "kv.write", false, None);
        parent.scopes = vec!["kv:a".into(), "kv:b".into()];
        let child = host.attenuate(&parent, "c", vec!["kv:a".into()]).unwrap();
        assert_eq!(child.scopes, vec!["kv:a".to_string()]);
        assert!(host.attenuate(&parent, "w", vec!["kv:z".into()]).is_err());
        assert!(host.attenuate(&parent, "w2", vec![]).is_err());
        assert!(host.attenuate(&parent, "", vec!["kv:a".into()]).is_err());
    }

    #[test]
    fn empty_idempotency_key_refuses() {
        let host = Host::with_clock(1000);
        let cap = host.mint("c-ik", "kv.write", false, None);
        let mut i = intent_write(cap, "a", json!(1));
        i.idempotency_key = Some(String::new());
        let r = host.submit(i);
        assert!(matches!(r.kind, ResultKind::AuthorityRefusal));
        assert!(r.ops.is_empty());
    }

    fn intent_delete(cap: Cap, key: &str, prior: Option<Value>) -> Intent {
        let mut args = BTreeMap::new();
        args.insert("key".into(), json!(key));
        if let Some(p) = prior {
            args.insert("prior".into(), p);
        }
        Intent {
            action: "kv.delete".into(),
            args,
            cap,
            trace: None,
            idempotency_key: None,
            activity_id: Some("act-del".into()),
        }
    }

    #[test]
    fn kv_delete_with_prior_reverses_to_set() {
        let host = Host::with_clock(1000);
        let cap = host.mint("c-delp", "kv.delete", false, None);
        let r = host.submit(intent_delete(cap, "k", Some(json!("old"))));
        assert!(matches!(r.kind, ResultKind::Ok));
        assert_eq!(r.ops[0].fq(), "kv.delete");
        assert_eq!(r.ops[0].payload.get("prior"), Some(&json!("old")));
        let rev = host.end_activity("act-del").unwrap();
        assert_eq!(rev.ops.len(), 1);
        assert_eq!(rev.ops[0].fq(), "kv.set");
        assert_eq!(rev.ops[0].payload.get("value"), Some(&json!("old")));
    }

    #[test]
    fn kv_delete_without_prior_is_non_reversible() {
        let host = Host::with_clock(1000);
        let cap = host.mint("c-deln", "kv.delete", false, None);
        let r = host.submit(intent_delete(cap, "k", None));
        assert!(matches!(r.kind, ResultKind::Ok));
        let rev = host.end_activity("act-del").unwrap();
        assert!(rev.ops.is_empty());
        assert!(!rev.non_reversible.is_empty());
    }

    const HMAC_KEY: [u8; 32] = [0x0b; 32];

    #[test]
    fn signed_cap_ok_unsigned_and_tamper_refuse() {
        let host = Host::with_clock(1000).with_hmac_key(HMAC_KEY);
        assert!(host.manifest().fail_closed.cap_signatures);
        let cap = host.mint("c-sig", "kv.write", false, None);
        assert!(cap.sig.as_deref().unwrap().starts_with("cek1:"));
        let r = host.submit(intent_write(cap.clone(), "a", json!(1)));
        assert!(matches!(r.kind, ResultKind::Ok));
        assert!(!r.ops.is_empty());

        let mut unsigned = cap.clone();
        unsigned.sig = None;
        let r = host.submit(intent_write(unsigned, "a", json!(1)));
        assert!(matches!(r.kind, ResultKind::AuthorityRefusal));
        assert!(r.ops.is_empty());

        let mut bad = cap;
        bad.sig = Some("cek1:00".into());
        let r = host.submit(intent_write(bad, "a", json!(1)));
        assert!(matches!(r.kind, ResultKind::AuthorityRefusal));
        assert!(r.ops.is_empty());
    }

    #[test]
    fn unsigned_host_still_accepts_legacy_caps() {
        let host = Host::with_clock(1000);
        assert!(!host.manifest().fail_closed.cap_signatures);
        let cap = host.mint("c-leg", "kv.write", false, None);
        assert!(cap.sig.is_none());
        let r = host.submit(intent_write(cap, "a", json!(1)));
        assert!(matches!(r.kind, ResultKind::Ok));
    }

    #[test]
    fn attenuate_resigns_child() {
        let host = Host::with_clock(1000).with_hmac_key(HMAC_KEY);
        let parent = host.mint("p", "kv.write", false, None);
        let child = host.attenuate(&parent, "c", vec!["kv:a".into()]).unwrap();
        assert_ne!(child.sig, parent.sig);
        assert!(cek_contract::cap_signature_valid(&HMAC_KEY, &child));
    }

    #[test]
    fn subject_bind_match_ok_mismatch_refuses() {
        let host = Host::with_clock(1000);
        let mut cap = host.mint("c-sub", "kv.write", false, None);
        cap.subject = Some("alice".into());
        let mut i = intent_write(cap.clone(), "a", json!(1));
        i.args.insert("subject".into(), json!("alice"));
        let r = host.submit(i);
        assert!(matches!(r.kind, ResultKind::Ok));

        let mut miss = intent_write(cap.clone(), "a", json!(1));
        miss.args.insert("subject".into(), json!("bob"));
        let r = host.submit(miss);
        assert!(matches!(r.kind, ResultKind::AuthorityRefusal));
        assert!(r.ops.is_empty());

        let r = host.submit(intent_write(cap, "a", json!(1)));
        assert!(matches!(r.kind, ResultKind::AuthorityRefusal));
        assert!(r.ops.is_empty());
    }

    #[test]
    fn empty_cap_subject_refuses() {
        let host = Host::with_clock(1000);
        let mut cap = host.mint("c-es", "kv.write", false, None);
        cap.subject = Some("  ".into());
        let r = host.submit(intent_write(cap, "a", json!(1)));
        assert!(matches!(r.kind, ResultKind::AuthorityRefusal));
        assert!(r.ops.is_empty());
    }

    const ED_SEED: [u8; 32] = [0x42; 32];

    #[test]
    fn ed25519_signed_ok_unsigned_and_tamper_refuse() {
        let host = Host::with_clock(1000).with_ed25519(ED_SEED);
        assert!(host.manifest().fail_closed.cap_signatures);
        let cap = host.mint("c-ed", "kv.write", false, None);
        assert!(cap.sig.as_deref().unwrap().starts_with("ed25519:"));
        let r = host.submit(intent_write(cap.clone(), "a", json!(1)));
        assert!(matches!(r.kind, ResultKind::Ok));

        let mut unsigned = cap.clone();
        unsigned.sig = None;
        let r = host.submit(intent_write(unsigned, "a", json!(1)));
        assert!(matches!(r.kind, ResultKind::AuthorityRefusal));
        assert!(r.ops.is_empty());

        let mut bad = cap;
        bad.sig = Some("ed25519:00".into());
        let r = host.submit(intent_write(bad, "a", json!(1)));
        assert!(matches!(r.kind, ResultKind::AuthorityRefusal));
        assert!(r.ops.is_empty());
    }

    #[test]
    fn ed25519_rotation_window_accepts_old_pub() {
        let a = Host::with_clock(1000).with_ed25519([0x01; 32]);
        let cap = a.mint("c-rot", "kv.write", false, None);
        let pub_a = a.ed25519_public().unwrap();
        // New Host mints with B but still trusts A (dual-speak).
        let b = Host::with_clock(1000)
            .with_ed25519([0x02; 32])
            .trust_ed25519(pub_a)
            .unwrap();
        let r = b.submit(intent_write(cap, "a", json!(1)));
        assert!(matches!(r.kind, ResultKind::Ok));
    }

    #[test]
    fn ed25519_untrusted_pub_refuses() {
        let a = Host::with_clock(1000).with_ed25519([0x01; 32]);
        let cap = a.mint("c-un", "kv.write", false, None);
        let b = Host::with_clock(1000).with_ed25519([0x02; 32]);
        let r = b.submit(intent_write(cap, "a", json!(1)));
        assert!(matches!(r.kind, ResultKind::AuthorityRefusal));
        assert!(r.ops.is_empty());
    }

    #[test]
    fn dual_speak_accepts_previous_refuses_unknown() {
        let host = Host::with_clock(1000)
            .accept_generation("cek-law-0")
            .unwrap();
        assert!(host
            .manifest()
            .accepted_generations
            .contains(&"cek-law-0".into()));
        let mut cap = host.mint("c-ds", "kv.write", false, None);
        cap.law_generation = Some("cek-law-0".into());
        let r = host.submit(intent_write(cap, "a", json!(1)));
        assert!(matches!(r.kind, ResultKind::Ok));

        let mut bad = host.mint("c-ds2", "kv.write", false, None);
        bad.law_generation = Some("cek-law-99".into());
        let r = host.submit(intent_write(bad, "a", json!(1)));
        assert!(matches!(r.kind, ResultKind::AuthorityRefusal));
        assert!(r.ops.is_empty());
    }

    /// LAW §4: dispatch → record lineage → project. Probe is filled in `dispatch_and_finish`.
    #[test]
    fn law4_records_lineage_before_project() {
        let _ = law4_take();
        let host = Host::with_clock(1000);
        let cap = host.mint("c-law4", "kv.write", false, None);
        let r = host.submit(intent_write(cap, "greeting", json!("hello")));
        assert!(matches!(r.kind, ResultKind::Ok));
        assert_eq!(r.ops.len(), 1);
        assert_eq!(r.ops[0].fq(), "kv.set");

        let steps = law4_take();
        let lin = steps
            .iter()
            .position(|s| *s == "record_lineage")
            .expect("lineage must be recorded");
        let proj = steps
            .iter()
            .position(|s| *s == "project")
            .expect("project must run");
        assert!(
            lin < proj,
            "LAW §4: record lineage before project, got {steps:?}"
        );
        assert_eq!(steps, ["dispatch", "record_lineage", "project"]);

        let entries = host.lineage_store().for_activity("act-1").unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].authorized_ops, r.ops);
    }

    /// Refuse path still returns Result with empty Ops (LAW: verify fail → no mutate Ops).
    #[test]
    fn law4_refuse_still_zero_ops() {
        let host = Host::with_clock(1000);
        let cap = host.mint("c-law4-refuse", "kv.read", false, None);
        let r = host.submit(intent_write(cap, "a", json!(1)));
        assert!(matches!(r.kind, ResultKind::AuthorityRefusal));
        assert!(r.ops.is_empty());
        assert!(host
            .lineage_store()
            .for_activity("act-1")
            .unwrap()
            .is_empty());
    }

    /// Source order in `dispatch_and_finish`: `lineage.commit` before `ResultMsg::ok`.
    #[test]
    fn law4_dispatch_and_finish_source_records_lineage_before_result_ok() {
        let src = include_str!("host.rs");
        let start = src
            .find("fn dispatch_and_finish")
            .expect("dispatch_and_finish");
        let rest = &src[start..];
        let end = rest
            .find("\n    pub fn report_receipt")
            .unwrap_or(rest.len());
        let body = &rest[..end];
        let lineage = body
            .find("self.lineage.commit")
            .expect("lineage.commit in dispatch_and_finish");
        let project = body
            .find("ResultMsg::ok")
            .expect("ResultMsg::ok project in dispatch_and_finish");
        assert!(
            lineage < project,
            "LAW §4: lineage.commit must appear before ResultMsg::ok in dispatch_and_finish"
        );
    }
}
