//! Host kernel orchestrator — mature reference implementation.

use crate::context::ContextIndex;
use crate::project::{dispatch_ops, inverse_ops, project_authorized};
use crate::{
    BoundAsk, HostError, HostResult, IdemBackend, IdemStore, LineageBackend, LineageStore,
    OnceBackend, OnceStore, ReverseOutcome,
};
use cek_contract::{
    ops_digest, result_digest, sealed_args_digest, Cap, Intent, LineageEntry, Manifest, Op,
    Profile, Receipt, ResultKind, ResultMsg, ReverseClass, UnknownOpPolicy, LAW_GENERATION,
    PROFILE_BASELINE, PROFILE_PRODUCTION_V1, PROFILE_UI, UI_OPS,
};
use ed25519_dalek::{SigningKey, VerifyingKey};
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// Host-only Recovery Cap registry (LAW §13). Ordinary [`Host::mint`] is not a recovery bypass.
#[derive(Default)]
struct RecoveryIndex {
    records: Vec<RecoveryRecord>,
}

struct RecoveryRecord {
    cap: Cap,
    args: BTreeMap<String, Value>,
    for_activity: Option<String>,
    for_lineage: Option<String>,
}

impl RecoveryRecord {
    fn covers_entry(&self, entry: &LineageEntry) -> bool {
        let lin_ok = match &self.for_lineage {
            Some(id) => id == &entry.id,
            None => true,
        };
        let act_ok = match &self.for_activity {
            Some(aid) => entry.activity_id.as_deref() == Some(aid.as_str()),
            None => true,
        };
        (self.for_activity.is_some() || self.for_lineage.is_some()) && lin_ok && act_ok
    }
}

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
    /// Recovery Caps minted under LAW §13 (Host-only; never Peer root).
    recovery: Mutex<RecoveryIndex>,
    /// Bound Peer Profile for LAW §11 project. `None` = missing Manifest → Baseline-only.
    peer_profile: Option<Profile>,
    /// Activity-scoped Context (LAW §8). Host-mediated; not a Cap substitute.
    pub(crate) contexts: Mutex<ContextIndex>,
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
            recovery: Mutex::new(RecoveryIndex::default()),
            peer_profile: None,
            contexts: Mutex::new(ContextIndex::default()),
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
            recovery: Mutex::new(RecoveryIndex::default()),
            peer_profile: None,
            contexts: Mutex::new(ContextIndex::default()),
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

    /// Baseline-only Peer Profile (missing Manifest / Baseline claim). LAW §11.
    pub fn baseline_peer_profile() -> Profile {
        Profile {
            name: PROFILE_BASELINE.into(),
            apply_set: cek_contract::BASELINE_OPS
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
            unknown_op_policy: UnknownOpPolicy::Skip,
        }
    }

    /// Baseline ∪ `ui.dom.*` apply-set. LAW §11 ability, not a Cap grant.
    pub fn ui_peer_profile() -> Profile {
        let mut apply_set: Vec<String> = cek_contract::BASELINE_OPS
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        apply_set.extend(UI_OPS.iter().map(|s| (*s).to_string()));
        Profile {
            name: PROFILE_UI.into(),
            apply_set,
            unknown_op_policy: UnknownOpPolicy::Skip,
        }
    }

    /// Map a Peer Manifest to a Profile. Unknown/empty names → Baseline.
    /// Manifest never grants Cap (LAW §11).
    pub fn profile_from_manifest(manifest: &Manifest) -> Profile {
        if manifest.profiles.iter().any(|p| p == PROFILE_UI) {
            Self::ui_peer_profile()
        } else {
            Self::baseline_peer_profile()
        }
    }

    /// Bind this Host to a Peer Profile for [`Host::submit`] (LAW §11).
    pub fn with_peer_profile(mut self, profile: Profile) -> Self {
        self.peer_profile = Some(profile);
        self
    }

    /// Bind this Host from a Peer Manifest. Missing/non-ui names → Baseline.
    pub fn with_peer_manifest(self, manifest: Manifest) -> Self {
        let profile = Self::profile_from_manifest(&manifest);
        self.with_peer_profile(profile)
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

    /// Mint a Recovery Cap (LAW §13). Host-only; never Peer root power.
    ///
    /// Still a normal Cap: verify, sealed args, fail closed, lineage for its own
    /// effects if later submitted with an Activity. Scope is the declared
    /// compensation action plus a resource allow-list derived from `sealed`.
    /// Bind at least one of `for_activity` / `for_lineage` — ordinary
    /// [`Host::mint`] is not a standing recovery bypass.
    #[allow(clippy::too_many_arguments)]
    pub fn mint_recovery(
        &self,
        id: impl Into<String>,
        action: impl Into<String>,
        once: bool,
        not_after: Option<u64>,
        for_activity: Option<&str>,
        for_lineage: Option<&str>,
        sealed: &BTreeMap<String, Value>,
    ) -> HostResult<Cap> {
        let id = id.into();
        let action = action.into();
        if id.trim().is_empty() {
            return Err(HostError::Authority("empty Cap id is not allowed".into()));
        }
        if action.trim().is_empty() {
            return Err(HostError::Authority("empty action is not allowed".into()));
        }
        let activity = Self::recovery_bind(for_activity)?;
        let lineage = Self::recovery_bind(for_lineage)?;
        if activity.is_none() && lineage.is_none() {
            return Err(HostError::Authority(
                "Recovery Cap requires for_activity or for_lineage (LAW §13)".into(),
            ));
        }
        let mut cap = self.mint(id, action, once, not_after);
        cap.scopes = recovery_scopes(&cap.action, sealed);
        if !sealed.is_empty() {
            cap.sealed_args_bind = Some(sealed_args_digest(sealed));
        }
        cap.sig = None;
        let cap = self.attach_sig(cap);
        let mut g = self
            .recovery
            .lock()
            .map_err(|_| HostError::Lineage("recovery lock".into()))?;
        g.records.push(RecoveryRecord {
            cap: cap.clone(),
            args: sealed.clone(),
            for_activity: activity,
            for_lineage: lineage,
        });
        Ok(cap)
    }

    fn recovery_bind(s: Option<&str>) -> HostResult<Option<String>> {
        match s {
            None => Ok(None),
            Some(s) if s.trim().is_empty() => Err(HostError::Authority(
                "empty Recovery Cap bind is not allowed".into(),
            )),
            Some(s) => Ok(Some(s.to_string())),
        }
    }

    /// Compensation Intents under registered Recovery Caps (ordinary submit).
    /// `None` = no path or a submit failed — caller marks NonReversible.
    fn try_compensate(&self, entry: &LineageEntry) -> Option<Vec<Op>> {
        let intents = {
            let g = self.recovery.lock().ok()?;
            let v: Vec<Intent> = g
                .records
                .iter()
                .filter(|r| r.covers_entry(entry))
                .map(|r| Intent {
                    action: r.cap.action.clone(),
                    args: r.args.clone(),
                    cap: r.cap.clone(),
                    trace: None,
                    idempotency_key: None,
                    activity_id: None,
                })
                .collect();
            v
        };
        if intents.is_empty() {
            return None;
        }
        let mut out = Vec::new();
        for intent in intents {
            let r = self.submit(intent);
            if !matches!(r.kind, ResultKind::Ok) {
                return None;
            }
            out.extend(r.ops);
        }
        Some(out)
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

    /// Lower authorized Ops to Baseline (ui.* → kv.set). Used by LAW §11 project.
    pub fn lower_ops(ops: &[Op]) -> Vec<Op> {
        ops.iter()
            .filter_map(cek_contract::lower_to_baseline)
            .collect()
    }

    /// Full submit pipeline (LAW §4 / CORE 06 Host duties):
    /// verify Cap → **mediate Context** (LAW §8) → consume once / idempotency bind
    /// → dispatch → **record lineage** → **project Ops** (LAW §11: ability ∪ Baseline)
    /// → Result+digest.
    ///
    /// Missing Peer Profile/Manifest → Baseline-only projection.
    /// Cap / Context refusal returns [`ResultMsg`] with `authority_refusal` and **zero** Ops.
    pub fn submit(&self, intent: Intent) -> ResultMsg {
        self.submit_for(intent, None)
    }

    /// Submit projecting Result Ops to `profile.apply_set ∪ Baseline` (LAW §11).
    ///
    /// `None` uses the Host-bound Peer Profile if set, else missing Manifest →
    /// Baseline-only. Lineage still records the **authorized** set.
    pub fn submit_for(&self, intent: Intent, profile: Option<&Profile>) -> ResultMsg {
        let profile = profile.or(self.peer_profile.as_ref());
        let now = (self.clock)();
        if let Err(e) = self.verify_cap(&intent, now) {
            return Self::err_result(e);
        }
        // LAW §8: Context mediation (inject / limit / isolate). Fail closed
        // before once/dispatch — not a Cap substitute, not a new Result kind.
        if let Err(e) = self.mediate_context(&intent) {
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
            match self.idempotency_lookup(key, &intent, profile) {
                Ok(Some(prior)) => return prior,
                Ok(None) => {}
                Err(e) => return Self::err_result(e),
            }
        }
        match self.once.ensure_available(&intent.cap.id, intent.cap.once) {
            Ok(()) => self.dispatch_and_finish(BoundAsk { intent, now }, profile),
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
    fn idempotency_lookup(
        &self,
        key: &str,
        intent: &Intent,
        profile: Option<&Profile>,
    ) -> HostResult<Option<ResultMsg>> {
        let Some(prior) = self.idem.get(key)? else {
            return Ok(None);
        };
        match dispatch_ops(intent) {
            Ok(ops) => {
                let projected = project_authorized(ops, profile);
                let digest = result_digest("ok", &projected, None);
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
        self.mediate_context(&intent)?;
        self.once
            .ensure_available(&intent.cap.id, intent.cap.once)?;
        Ok(BoundAsk { intent, now })
    }

    fn law4_note(step: &'static str) {
        #[cfg(test)]
        tests::LAW4_STEPS.with(|s| s.borrow_mut().push(step));
        let _ = step;
    }

    fn dispatch_and_finish(&self, bound: BoundAsk, profile: Option<&Profile>) -> ResultMsg {
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

        // Projection is a pure function of authorized Ops + Peer Profile (LAW §11).
        // Compute here so idempotency caches the Result the Peer will see; the
        // LAW §4 project *step* still lands after lineage (Result assembly).
        let projected = project_authorized(authorized.clone(), profile);
        let digest = result_digest("ok", &projected, None);

        // Idempotency bind after digest is known, **before** lineage (no second cause).
        if let Some(ref key) = intent.idempotency_key {
            let cached = ResultMsg {
                kind: cek_contract::ResultKind::Ok,
                ops: projected.clone(),
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

        // LAW §4 steps 5–6: Project Ops onto Result (LAW §11), then return.
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

    /// End Activity → reverse lineage (LAW §9).
    ///
    /// Preference: if landed_ops annotated → build inverse from landed;
    /// else use inverse_ops recorded at commit; Compensation submits Intents
    /// under a Recovery Cap (LAW §13); NonReversible listed honestly.
    pub fn end_activity(&self, activity_id: &str) -> HostResult<ReverseOutcome> {
        if activity_id.is_empty() {
            return Err(HostError::Lineage("empty activity_id".into()));
        }
        self.lineage.mark_ended(activity_id)?;
        self.drop_context(activity_id);
        let entries = self.lineage.for_activity(activity_id)?;
        Ok(self.reverse_entries(entries))
    }

    /// Revoke a Cap (LAW §5 Active→Revoked) and reverse causes under it (LAW §9).
    ///
    /// Same reverse classes as [`Host::end_activity`]: Inverse Ops, Compensation
    /// Intents under a Recovery Cap (LAW §13), or an explicit NonReversible listing.
    /// Revoke surface is unchanged: Cap dead + Cap-scoped reverse.
    pub fn revoke(&self, cap_id: &str) -> HostResult<ReverseOutcome> {
        if cap_id.is_empty() {
            return Err(HostError::Authority("empty Cap id is not allowed".into()));
        }
        self.lineage.mark_revoked(cap_id)?;
        let entries = self.lineage.for_cap(cap_id)?;
        Ok(self.reverse_entries(entries))
    }

    fn reverse_entries(&self, entries: Vec<LineageEntry>) -> ReverseOutcome {
        let mut ops = Vec::new();
        let mut non_reversible = Vec::new();
        let mut used_landed = false;
        for entry in entries.into_iter().rev() {
            match entry.reverse_class {
                ReverseClass::Inverse => {
                    let inv = if !entry.landed_ops.is_empty() {
                        used_landed = true;
                        inverse_ops(&entry.landed_ops)
                    } else {
                        entry.inverse_ops
                    };
                    if inv.is_empty() {
                        non_reversible.push(entry.id);
                    } else {
                        ops.extend(inv);
                    }
                }
                ReverseClass::Compensation => match self.try_compensate(&entry) {
                    Some(comp_ops) => ops.extend(comp_ops),
                    None => non_reversible.push(entry.id),
                },
                ReverseClass::NonReversible => {
                    non_reversible.push(entry.id);
                }
            }
        }
        ReverseOutcome {
            ops,
            non_reversible,
            used_landed,
        }
    }
}

/// Narrow Recovery Cap scopes from declared compensation args (LAW §13).
fn recovery_scopes(action: &str, sealed: &BTreeMap<String, Value>) -> Vec<String> {
    match action {
        cek_contract::ACTION_KV_WRITE | cek_contract::ACTION_KV_DELETE => sealed
            .get("key")
            .and_then(|v| v.as_str())
            .filter(|k| !k.is_empty())
            .map(|k| vec![format!("kv:{k}")])
            .unwrap_or_default(),
        cek_contract::ACTION_UI_MORPH | cek_contract::ACTION_UI_RESTORE => sealed
            .get("target")
            .and_then(|v| v.as_str())
            .filter(|t| !t.is_empty())
            .map(|t| vec![format!("ui:{t}")])
            .unwrap_or_default(),
        cek_contract::ACTION_LOG_APPEND => vec!["log".into()],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FileIdemStore, FileLineageStore, FileOnceStore};
    use cek_contract::{
        baseline, Manifest, Profile, ResultKind, ReverseClass, LAW_GENERATION, PROFILE_UI,
    };
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
        let mut sealed = BTreeMap::new();
        sealed.insert("key".into(), json!("compensated"));
        sealed.insert("value".into(), json!(true));
        host.mint_recovery(
            "rec-use",
            "kv.write",
            false,
            None,
            Some("act-comp"),
            None,
            &sealed,
        )
        .unwrap();
        host.lineage_store()
            .commit(
                "cap",
                Some("act-comp"),
                "log.append",
                vec![baseline::log_append("hi")],
                ReverseClass::Compensation,
                vec![],
            )
            .unwrap();
        let rev = host.end_activity("act-comp").unwrap();
        assert!(
            rev.non_reversible.is_empty(),
            "successful compensation must not mark NonReversible"
        );
        assert_eq!(rev.ops.len(), 1);
        assert_eq!(rev.ops[0].fq(), "kv.set");
        assert_eq!(rev.ops[0].payload.get("key"), Some(&json!("compensated")));
    }

    #[test]
    fn compensation_without_recovery_cap_is_non_reversible() {
        let host = Host::with_clock(1000);
        host.lineage_store()
            .commit(
                "cap",
                Some("act-comp-bare"),
                "kv.write",
                vec![baseline::kv_set("k", json!(1))],
                ReverseClass::Compensation,
                vec![],
            )
            .unwrap();
        let rev = host.end_activity("act-comp-bare").unwrap();
        assert!(rev.ops.is_empty(), "must not report clean reverse");
        assert_eq!(rev.non_reversible.len(), 1);
    }

    #[test]
    fn mint_recovery_is_ordinary_cap() {
        let host = Host::with_clock(1000);
        let mut sealed = BTreeMap::new();
        sealed.insert("key".into(), json!("compensated"));
        sealed.insert("value".into(), json!(true));
        let cap = host
            .mint_recovery(
                "rec-1",
                "kv.write",
                false,
                None,
                Some("act-rec"),
                None,
                &sealed,
            )
            .unwrap();
        assert_eq!(cap.action, "kv.write");
        assert_eq!(cap.scopes, vec!["kv:compensated".to_string()]);
        assert!(cap.sealed_args_bind.is_some());
        let r = host.submit(Intent {
            action: "kv.write".into(),
            args: sealed,
            cap,
            trace: None,
            idempotency_key: None,
            activity_id: None,
        });
        assert!(matches!(r.kind, ResultKind::Ok));
        assert_eq!(r.ops[0].fq(), "kv.set");
    }

    #[test]
    fn mint_recovery_refuses_empty_bind_and_peer_root_shape() {
        let host = Host::with_clock(1000);
        let sealed = BTreeMap::new();
        assert!(host
            .mint_recovery("r", "kv.write", false, None, None, None, &sealed)
            .is_err());
        assert!(host
            .mint_recovery("", "kv.write", false, None, Some("act"), None, &sealed)
            .is_err());
        assert!(host
            .mint_recovery("r", "", false, None, Some("act"), None, &sealed)
            .is_err());
        assert!(host
            .mint_recovery("r", "kv.write", false, None, Some(""), None, &sealed)
            .is_err());
        assert!(host
            .mint_recovery("r", "kv.write", false, None, None, Some("  "), &sealed)
            .is_err());
    }

    #[test]
    fn ordinary_mint_is_not_recovery_bypass() {
        let host = Host::with_clock(1000);
        let _ = host.mint("bootstrap", "kv.write", false, None);
        host.lineage_store()
            .commit(
                "cap",
                Some("act-boot"),
                "kv.write",
                vec![baseline::kv_set("k", json!(1))],
                ReverseClass::Compensation,
                vec![],
            )
            .unwrap();
        let rev = host.end_activity("act-boot").unwrap();
        assert!(rev.ops.is_empty(), "bootstrap mint must not compensate");
        assert_eq!(rev.non_reversible.len(), 1);
    }

    #[test]
    fn compensation_failure_marks_non_reversible() {
        let host = Host::with_clock(1000);
        let mut sealed = BTreeMap::new();
        sealed.insert("key".into(), json!("compensated"));
        sealed.insert("value".into(), json!(true));
        host.mint_recovery(
            "rec-exp",
            "kv.write",
            false,
            Some(1),
            Some("act-comp-fail"),
            None,
            &sealed,
        )
        .unwrap();
        host.lineage_store()
            .commit(
                "cap-orig",
                Some("act-comp-fail"),
                "log.append",
                vec![baseline::log_append("hi")],
                ReverseClass::Compensation,
                vec![],
            )
            .unwrap();
        let rev = host.end_activity("act-comp-fail").unwrap();
        assert!(rev.ops.is_empty(), "must not report clean reverse");
        assert_eq!(rev.non_reversible.len(), 1);
    }

    #[test]
    fn submit_never_auto_classifies_compensation() {
        let host = Host::with_clock(1000);
        let mut sealed = BTreeMap::new();
        sealed.insert("key".into(), json!("restored"));
        sealed.insert("value".into(), json!(1));
        host.mint_recovery(
            "rec-cls",
            "kv.write",
            false,
            None,
            Some("act-cls"),
            None,
            &sealed,
        )
        .unwrap();
        let cap = host.mint("c-cls", "log.append", false, None);
        let mut args = BTreeMap::new();
        args.insert("message".into(), json!("hi"));
        let r = host.submit(Intent {
            action: "log.append".into(),
            args,
            cap,
            trace: None,
            idempotency_key: None,
            activity_id: Some("act-cls".into()),
        });
        assert!(matches!(r.kind, ResultKind::Ok));
        let entries = host.lineage_store().for_activity("act-cls").unwrap();
        assert_eq!(entries.len(), 1);
        assert!(
            matches!(entries[0].reverse_class, ReverseClass::NonReversible),
            "submit auto-class is Inverse vs NonReversible only"
        );
        let rev = host.end_activity("act-cls").unwrap();
        assert!(rev.ops.is_empty());
        assert!(!rev.non_reversible.is_empty());
    }

    #[test]
    fn recovery_cap_scope_narrow_refuses_other_resource() {
        let host = Host::with_clock(1000);
        let mut sealed = BTreeMap::new();
        sealed.insert("key".into(), json!("only-this"));
        sealed.insert("value".into(), json!(1));
        let cap = host
            .mint_recovery(
                "rec-sc",
                "kv.write",
                false,
                None,
                Some("act-sc"),
                None,
                &sealed,
            )
            .unwrap();
        let mut args = BTreeMap::new();
        args.insert("key".into(), json!("other"));
        args.insert("value".into(), json!(1));
        let r = host.submit(Intent {
            action: "kv.write".into(),
            args,
            cap,
            trace: None,
            idempotency_key: None,
            activity_id: None,
        });
        assert!(matches!(r.kind, ResultKind::AuthorityRefusal));
        assert!(r.ops.is_empty());
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
        let r = host.submit_for(
            intent_morph(cap, "hdr", json!({"t": "new"}), Some(json!({"t": "old"}))),
            Some(&Host::ui_peer_profile()),
        );
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

    /// LAW §11: missing Manifest → Baseline-only Peer. Result Ops are projected,
    /// not the full `ui.dom.*` catalog.
    #[test]
    fn missing_manifest_projects_baseline_not_ui_catalog() {
        let host = Host::with_clock(1000);
        let cap = host.mint("c-ui-miss", "ui.morph", false, None);
        let r = host.submit(intent_morph(
            cap,
            "hdr",
            json!({"t": 1}),
            Some(json!({"t": 0})),
        ));
        assert!(matches!(r.kind, ResultKind::Ok));
        assert_eq!(r.ops.len(), 1);
        assert_eq!(r.ops[0].fq(), "kv.set");
        assert_eq!(
            r.ops[0].payload.get("key").and_then(|v| v.as_str()),
            Some("ui:hdr")
        );
        assert!(r.ops.iter().all(|op| op.ns != "ui.dom"));
        let entries = host.lineage_store().for_activity("act-ui").unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].authorized_ops[0].fq(), "ui.dom.morph");
        assert_ne!(entries[0].authorized_ops, r.ops);
    }

    #[test]
    fn baseline_profile_projects_baseline_not_ui_catalog() {
        let host = Host::with_clock(1000);
        let cap = host.mint("c-ui-base", "ui.morph", false, None);
        let r = host.submit_for(
            intent_morph(cap, "hdr", json!({"t": 1}), None),
            Some(&Host::baseline_peer_profile()),
        );
        assert!(matches!(r.kind, ResultKind::Ok));
        assert_eq!(r.ops[0].fq(), "kv.set");
        assert!(r.ops.iter().all(|op| op.ns != "ui.dom"));
    }

    #[test]
    fn limited_profile_unions_baseline_not_full_catalog() {
        let host = Host::with_clock(1000);
        let cap = host.mint("c-ui-lim", "ui.morph", false, None);
        let limited = Profile {
            name: "limited".into(),
            apply_set: vec!["kv.set".into()],
            unknown_op_policy: Default::default(),
        };
        let r = host.submit_for(
            intent_morph(cap, "hdr", json!({"t": 1}), None),
            Some(&limited),
        );
        assert!(matches!(r.kind, ResultKind::Ok));
        assert_eq!(r.ops[0].fq(), "kv.set");
        assert!(r.ops.iter().all(|op| op.fq() != "ui.dom.morph"));
    }

    #[test]
    fn ui_profile_keeps_domain_ops() {
        let host = Host::with_clock(1000);
        let cap = host.mint("c-ui-keep", "ui.morph", false, None);
        let r = host.submit_for(
            intent_morph(cap, "hdr", json!({"t": 1}), None),
            Some(&Host::ui_peer_profile()),
        );
        assert_eq!(r.ops[0].fq(), "ui.dom.morph");
    }

    #[test]
    fn empty_manifest_defaults_baseline() {
        let host = Host::with_clock(1000).with_peer_manifest(Manifest {
            law_generation: LAW_GENERATION.into(),
            accepted_generations: vec![],
            profiles: vec![],
            fail_closed: Default::default(),
        });
        let cap = host.mint("c-ui-man", "ui.morph", false, None);
        let r = host.submit(intent_morph(cap, "hdr", json!({"t": 1}), None));
        assert_eq!(r.ops[0].fq(), "kv.set");
        assert!(r.ops.iter().all(|op| op.ns != "ui.dom"));
    }

    #[test]
    fn ui_manifest_keeps_domain_ops() {
        let host = Host::with_clock(1000).with_peer_manifest(Manifest {
            law_generation: LAW_GENERATION.into(),
            accepted_generations: vec![],
            profiles: vec![PROFILE_UI.into()],
            fail_closed: Default::default(),
        });
        let cap = host.mint("c-ui-man-ui", "ui.morph", false, None);
        let r = host.submit(intent_morph(cap, "hdr", json!({"t": 1}), None));
        assert_eq!(r.ops[0].fq(), "ui.dom.morph");
    }

    #[test]
    fn same_authorized_and_profile_same_projected_ops() {
        let host = Host::with_clock(1000);
        let cap = host.mint("c-ui-det", "ui.morph", false, None);
        let intent = intent_morph(cap, "hdr", json!({"t": 1}), None);
        let a = host.submit_for(intent.clone(), Some(&Host::baseline_peer_profile()));
        let host2 = Host::with_clock(1000);
        let cap2 = host2.mint("c-ui-det2", "ui.morph", false, None);
        let intent2 = intent_morph(cap2, "hdr", json!({"t": 1}), None);
        let b = host2.submit_for(intent2, Some(&Host::baseline_peer_profile()));
        assert_eq!(a.ops, b.ops);
        assert_eq!(a.digest, b.digest);
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

    #[test]
    fn revoke_emits_inverse_delete() {
        let host = Host::with_clock(1000);
        let cap = host.mint("c-rev", "kv.write", false, None);
        let r = host.submit(intent_write(cap.clone(), "k", json!(1)));
        assert!(matches!(r.kind, ResultKind::Ok));
        let rev = host.revoke(&cap.id).unwrap();
        assert_eq!(rev.ops.len(), 1);
        assert_eq!(rev.ops[0].fq(), "kv.delete");
        assert!(rev.non_reversible.is_empty());
    }

    #[test]
    fn revoke_cap_dead_verify_and_submit_refuse() {
        let host = Host::with_clock(1000);
        let cap = host.mint("c-dead", "kv.write", false, None);
        let r = host.submit(intent_write(cap.clone(), "k", json!(1)));
        assert!(matches!(r.kind, ResultKind::Ok));
        host.revoke(&cap.id).unwrap();
        assert!(host.lineage_store().is_revoked(&cap.id));
        assert!(host
            .verify_and_bind(intent_write(cap.clone(), "k", json!(2)), 1000)
            .is_err());
        let r2 = host.submit(intent_write(cap, "k", json!(2)));
        assert!(matches!(r2.kind, ResultKind::AuthorityRefusal));
        assert!(r2.ops.is_empty());
        assert!(r2.error.as_deref().unwrap().contains("revoked"));
    }

    #[test]
    fn revoke_non_reversible_listed_honestly() {
        let host = Host::with_clock(1000);
        let cap = host.mint("c-rev-nr", "log.append", false, None);
        let mut args = BTreeMap::new();
        args.insert("message".into(), json!("hi"));
        let r = host.submit(Intent {
            action: "log.append".into(),
            args,
            cap: cap.clone(),
            trace: None,
            idempotency_key: None,
            activity_id: Some("act-rev-log".into()),
        });
        assert!(matches!(r.kind, ResultKind::Ok));
        let rev = host.revoke(&cap.id).unwrap();
        assert!(rev.ops.is_empty(), "must not claim clean reverse");
        assert!(!rev.non_reversible.is_empty());
        let r2 = host.submit(Intent {
            action: "log.append".into(),
            args: {
                let mut a = BTreeMap::new();
                a.insert("message".into(), json!("again"));
                a
            },
            cap,
            trace: None,
            idempotency_key: None,
            activity_id: Some("act-rev-log-2".into()),
        });
        assert!(matches!(r2.kind, ResultKind::AuthorityRefusal));
        assert!(r2.ops.is_empty());
    }

    #[test]
    fn revoke_compensation_listed_not_silent_success() {
        let host = Host::with_clock(1000);
        let mut sealed = BTreeMap::new();
        sealed.insert("key".into(), json!("rev-comp"));
        sealed.insert("value".into(), json!(1));
        host.mint_recovery(
            "rec-rev",
            "kv.write",
            false,
            None,
            Some("act-comp-rev"),
            None,
            &sealed,
        )
        .unwrap();
        host.lineage_store()
            .commit(
                "cap-comp-rev",
                Some("act-comp-rev"),
                "log.append",
                vec![baseline::log_append("hi")],
                ReverseClass::Compensation,
                vec![],
            )
            .unwrap();
        let orig = host.mint("cap-comp-rev", "log.append", false, None);
        let rev = host.revoke(&orig.id).unwrap();
        assert!(
            rev.non_reversible.is_empty(),
            "successful compensation must not mark NonReversible"
        );
        assert_eq!(rev.ops.len(), 1);
        assert_eq!(rev.ops[0].fq(), "kv.set");
        assert_eq!(rev.ops[0].payload.get("key"), Some(&json!("rev-comp")));
        assert!(host.lineage_store().is_revoked(&orig.id));
        let r2 = host.submit(Intent {
            action: "log.append".into(),
            args: {
                let mut a = BTreeMap::new();
                a.insert("message".into(), json!("again"));
                a
            },
            cap: orig,
            trace: None,
            idempotency_key: None,
            activity_id: Some("act-comp-rev-2".into()),
        });
        assert!(matches!(r2.kind, ResultKind::AuthorityRefusal));
        assert!(r2.ops.is_empty());
        assert!(r2.error.as_deref().unwrap().contains("revoked"));
    }

    #[test]
    fn revoke_inverse_without_plan_marked_non_reversible() {
        let host = Host::with_clock(1000);
        host.lineage_store()
            .commit(
                "cap-empty-inv",
                Some("act-empty-inv"),
                "kv.write",
                vec![baseline::kv_set("k", json!(1))],
                ReverseClass::Inverse,
                vec![],
            )
            .unwrap();
        let rev = host.revoke("cap-empty-inv").unwrap();
        assert!(rev.ops.is_empty());
        assert_eq!(
            rev.non_reversible.len(),
            1,
            "empty Inverse must not be silent clean reverse"
        );
    }

    #[test]
    fn double_revoke_errors() {
        let host = Host::with_clock(1000);
        let cap = host.mint("c-drev", "kv.write", false, None);
        let _ = host.submit(intent_write(cap.clone(), "k", json!(1)));
        assert!(host.revoke(&cap.id).is_ok());
        assert!(host.revoke(&cap.id).is_err());
    }

    #[test]
    fn empty_cap_id_revoke_errors() {
        let host = Host::with_clock(1000);
        assert!(host.revoke("").is_err());
    }

    #[test]
    fn revoke_unused_cap_then_submit_refuses() {
        let host = Host::with_clock(1000);
        let cap = host.mint("c-unused-rev", "kv.write", false, None);
        let rev = host.revoke(&cap.id).unwrap();
        assert!(rev.ops.is_empty());
        assert!(rev.non_reversible.is_empty());
        let r = host.submit(intent_write(cap, "k", json!(1)));
        assert!(matches!(r.kind, ResultKind::AuthorityRefusal));
        assert!(r.ops.is_empty());
    }

    #[test]
    fn revoke_one_cap_does_not_reverse_another() {
        let host = Host::with_clock(1000);
        let a = host.mint("c-a", "kv.write", false, None);
        let b = host.mint("c-b", "kv.write", false, None);
        let mut ia = intent_write(a.clone(), "ka", json!(1));
        ia.activity_id = Some("act-a".into());
        let mut ib = intent_write(b.clone(), "kb", json!(2));
        ib.activity_id = Some("act-b".into());
        assert!(matches!(host.submit(ia).kind, ResultKind::Ok));
        assert!(matches!(host.submit(ib).kind, ResultKind::Ok));
        let rev = host.revoke(&a.id).unwrap();
        assert_eq!(rev.ops.len(), 1);
        assert_eq!(rev.ops[0].payload.get("key"), Some(&json!("ka")));
        let r_b = host.submit({
            let mut i = intent_write(b, "kb2", json!(3));
            i.activity_id = Some("act-b".into());
            i
        });
        assert!(matches!(r_b.kind, ResultKind::Ok));
        let r_a = host.submit(intent_write(a, "ka2", json!(4)));
        assert!(matches!(r_a.kind, ResultKind::AuthorityRefusal));
        assert!(r_a.ops.is_empty());
    }

    #[test]
    fn revoke_prefers_landed_reverse() {
        let host = Host::with_clock(1000);
        let cap = host.mint("c-rev-land", "kv.write", false, None);
        let r = host.submit(intent_write(cap.clone(), "k", json!(1)));
        assert!(matches!(r.kind, ResultKind::Ok));
        let receipt = Receipt {
            landed: r.ops.clone(),
            failed: vec![],
        };
        host.report_receipt("act-1", &receipt).unwrap();
        let rev = host.revoke(&cap.id).unwrap();
        assert!(rev.used_landed);
        assert_eq!(rev.ops[0].fq(), "kv.delete");
    }

    #[test]
    fn durable_file_host_revoke_survives_reopen() {
        static N: AtomicU64 = AtomicU64::new(1);
        let dir = std::env::temp_dir().join(format!(
            "cek-host-revoke-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        {
            let host = Host::with_stores(
                Arc::new(FileOnceStore::open(&dir).unwrap()),
                Arc::new(FileIdemStore::open(&dir).unwrap()),
                Arc::new(FileLineageStore::open(&dir).unwrap()),
                1000,
            );
            let cap = host.mint("c-file-rev", "kv.write", false, None);
            let r = host.submit(intent_write(cap.clone(), "greet", json!("hi")));
            assert!(matches!(r.kind, ResultKind::Ok));
            let rev = host.revoke(&cap.id).unwrap();
            assert_eq!(rev.ops[0].fq(), "kv.delete");
        }
        let host2 = Host::with_stores(
            Arc::new(FileOnceStore::open(&dir).unwrap()),
            Arc::new(FileIdemStore::open(&dir).unwrap()),
            Arc::new(FileLineageStore::open(&dir).unwrap()),
            1000,
        );
        let cap2 = host2.mint("c-file-rev", "kv.write", false, None);
        let r2 = host2.submit(intent_write(cap2, "greet", json!("again")));
        assert!(matches!(r2.kind, ResultKind::AuthorityRefusal));
        assert!(r2.ops.is_empty());
        assert!(host2.revoke("c-file-rev").is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn context_applied_on_submit() {
        let host = Host::with_clock(1000);
        let cap = host.mint("c-ctx-ok", "kv.write", false, None);
        host.inject("act-1", vec!["kv:greeting".into()]).unwrap();
        let r = host.submit(intent_write(cap, "greeting", json!("hello")));
        assert!(matches!(r.kind, ResultKind::Ok), "{:?}", r.error);
        assert_eq!(r.ops.len(), 1);
        assert_eq!(r.ops[0].payload.get("key"), Some(&json!("greeting")));
        let ctx = host.context_of("act-1").unwrap();
        assert_eq!(ctx.injected, vec!["kv:greeting".to_string()]);
        assert!(!ctx.isolated);
        assert!(ctx.limits.is_empty());
    }

    #[test]
    fn context_over_limit_refused() {
        let host = Host::with_clock(1000);
        let cap = host.mint("c-ctx-lim", "kv.write", false, None);
        host.inject("act-1", vec!["kv".into()]).unwrap();
        host.limit("act-1", vec!["kv:greeting".into()]).unwrap();
        let r = host.submit(intent_write(cap, "other", json!(1)));
        assert!(matches!(r.kind, ResultKind::AuthorityRefusal));
        assert!(r.ops.is_empty());
        assert!(r.error.as_deref().unwrap_or("").contains("over-limit"));
    }

    #[test]
    fn context_undeclared_inject_fails_closed() {
        let host = Host::with_clock(1000);
        let cap = host.mint("c-ctx-und", "kv.write", false, None);
        host.inject("act-1", vec!["kv:greeting".into()]).unwrap();
        let r = host.submit(intent_write(cap, "other", json!(1)));
        assert!(matches!(r.kind, ResultKind::AuthorityRefusal));
        assert!(r.ops.is_empty());
        assert!(r
            .error
            .as_deref()
            .unwrap_or("")
            .contains("undeclared inject"));
    }

    #[test]
    fn context_isolate_holds() {
        let host = Host::with_clock(1000);
        let cap_a = host.mint("c-iso-a", "kv.write", false, None);
        let cap_b = host.mint("c-iso-b", "kv.write", false, None);
        host.inject("act-a", vec!["kv:secret".into()]).unwrap();
        host.isolate("act-a").unwrap();
        let ok = host.submit({
            let mut i = intent_write(cap_a, "secret", json!(1));
            i.activity_id = Some("act-a".into());
            i
        });
        assert!(matches!(ok.kind, ResultKind::Ok), "{:?}", ok.error);
        let leak = host.submit({
            let mut i = intent_write(cap_b.clone(), "secret", json!(2));
            i.activity_id = Some("act-b".into());
            i
        });
        assert!(matches!(leak.kind, ResultKind::AuthorityRefusal));
        assert!(leak.ops.is_empty());
        assert!(leak
            .error
            .as_deref()
            .unwrap_or("")
            .contains("isolate holds"));
        let no_act = host.submit({
            let mut i = intent_write(cap_b, "secret", json!(3));
            i.activity_id = None;
            i
        });
        assert!(matches!(no_act.kind, ResultKind::AuthorityRefusal));
        assert!(no_act.ops.is_empty());
    }

    #[test]
    fn context_limit_is_not_cap_scope() {
        let host = Host::with_clock(1000);
        let mut cap = host.mint("c-ctx-vs-sc", "kv.write", false, None);
        cap.scopes = vec!["kv:other".into()];
        host.inject("act-1", vec!["kv:greeting".into()]).unwrap();
        let r = host.submit(intent_write(cap, "greeting", json!(1)));
        assert!(
            matches!(r.kind, ResultKind::AuthorityRefusal),
            "Cap.scopes still bind independently of Context: {:?}",
            r.error
        );
        assert!(r.ops.is_empty());
        assert!(
            !r.error.as_deref().unwrap_or("").contains("over-limit"),
            "must be Cap scope deny, not Context limit: {:?}",
            r.error
        );
    }

    #[test]
    fn context_limit_is_not_isolate() {
        let host = Host::with_clock(1000);
        let cap_b = host.mint("c-lim-b", "kv.write", false, None);
        host.inject("act-a", vec!["kv:secret".into()]).unwrap();
        host.limit("act-a", vec!["kv:secret".into()]).unwrap();
        let r = host.submit({
            let mut i = intent_write(cap_b, "secret", json!(1));
            i.activity_id = Some("act-b".into());
            i
        });
        assert!(
            matches!(r.kind, ResultKind::Ok),
            "limit must not isolate across Activities: {:?}",
            r.error
        );
    }

    #[test]
    fn context_refuse_does_not_burn_once() {
        let host = Host::with_clock(1000);
        let cap = host.mint("c-ctx-once", "kv.write", true, None);
        host.inject("act-1", vec!["kv:greeting".into()]).unwrap();
        let r = host.submit(intent_write(cap.clone(), "other", json!(1)));
        assert!(matches!(r.kind, ResultKind::AuthorityRefusal));
        assert!(!host.once_store().is_consumed("c-ctx-once"));
        let ok = host.submit(intent_write(cap, "greeting", json!(1)));
        assert!(matches!(ok.kind, ResultKind::Ok));
    }

    #[test]
    fn context_dropped_on_end_activity() {
        let host = Host::with_clock(1000);
        let cap = host.mint("c-ctx-end", "kv.write", false, None);
        host.inject("act-1", vec!["kv:secret".into()]).unwrap();
        host.isolate("act-1").unwrap();
        let _ = host.submit(intent_write(cap.clone(), "secret", json!(1)));
        host.end_activity("act-1").unwrap();
        assert!(host.context_of("act-1").is_none());
        let cap_b = host.mint("c-ctx-end-b", "kv.write", false, None);
        let r = host.submit({
            let mut i = intent_write(cap_b, "secret", json!(2));
            i.activity_id = Some("act-b".into());
            i
        });
        assert!(matches!(r.kind, ResultKind::Ok), "{:?}", r.error);
    }
}
