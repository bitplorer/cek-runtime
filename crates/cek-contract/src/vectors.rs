//! Conformance vector loading (JSON fixtures).

use crate::{ContractError, ContractResult, Intent, Op, ResultMsg};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// One executable conformance case.
///
/// Additive optional fields only — older fixtures remain valid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorCase {
    /// Stable case id.
    pub id: String,
    /// Family name (e.g. `cap_verify`, `baseline_apply`).
    pub family: String,
    /// Human description.
    pub description: String,
    /// Input Intent (when applicable).
    #[serde(default)]
    pub intent: Option<Intent>,
    /// Optional prior submit on the same Host (e.g. first once-Cap use).
    #[serde(default)]
    pub prior_intent: Option<Intent>,
    /// If true, `prior_intent` must conclude `ok`.
    #[serde(default)]
    pub prior_must_ok: bool,
    /// After `prior_intent`, end this Activity before the checked submit.
    #[serde(default)]
    pub prior_end_activity: Option<String>,
    /// Host clock used for expiry checks (unix seconds).
    #[serde(default)]
    pub now: Option<u64>,
    /// Expected Result kind string: ok | authority_refusal | dispatch_error.
    pub expect_kind: String,
    /// If true, expected Ops list must be empty.
    #[serde(default)]
    pub expect_ops_empty: bool,
    /// Optional exact expected Ops (JSON).
    #[serde(default)]
    pub expect_ops: Option<Vec<Op>>,
    /// After the checked submit, apply via Peer.
    #[serde(default)]
    pub peer_apply: bool,
    /// Skip Host submit and apply this Result via Peer (Peer-only case).
    #[serde(default)]
    pub peer_result: Option<ResultMsg>,
    /// Peer unknown-Op policy: `skip` (default) or `fail_batch`.
    #[serde(default)]
    pub peer_unknown_policy: Option<String>,
    /// Expected Peer kv after apply. JSON `null` means the key must be absent.
    #[serde(default)]
    pub expect_peer_kv: Option<BTreeMap<String, Value>>,
    /// After submit (and optional apply), report receipt for this Activity.
    #[serde(default)]
    pub report_receipt: bool,
    /// After submit / receipt, end this Activity and check reverse.
    #[serde(default)]
    pub end_activity: Option<String>,
    /// Expected reverse Ops from `end_activity`.
    #[serde(default)]
    pub expect_reverse_ops: Option<Vec<Op>>,
    /// Expected `used_landed` flag from reverse.
    #[serde(default)]
    pub expect_used_landed: Option<bool>,
    /// Call `end_activity` a second time (expects error).
    #[serde(default)]
    pub end_activity_again: bool,
    /// After the checked submit, whether `intent.cap.id` is consumed.
    #[serde(default)]
    pub expect_once_consumed: Option<bool>,
    /// Peer profile: `baseline` (default) or `ui`.
    #[serde(default)]
    pub peer_profile: Option<String>,
    /// Expected Peer UI targets after apply. JSON `null` means absent.
    #[serde(default)]
    pub expect_peer_ui: Option<BTreeMap<String, Value>>,
    /// Expected Baseline-lowered Ops of the Host Result (`ui.*` → `kv.set`).
    #[serde(default)]
    pub expect_lowered_ops: Option<Vec<Op>>,
    /// Hex-encoded 32-byte HMAC key. When set, Host requires Cap signatures.
    #[serde(default)]
    pub hmac_key: Option<String>,
    /// If true, runner attaches a valid Host HMAC before submit.
    #[serde(default)]
    pub sign_cap: bool,
    /// Hex-encoded 32-byte Ed25519 seed. When set, Host requires Ed25519 sigs.
    #[serde(default)]
    pub ed25519_seed: Option<String>,
    /// Extra law generations the Host accepts (dual-speak).
    #[serde(default)]
    pub accept_generations: Option<Vec<String>>,
    /// After `prior_intent` (and optional `prior_end_activity`), revoke this Cap.
    #[serde(default)]
    pub revoke_cap: Option<String>,
    /// Expected reverse Ops from `revoke_cap`.
    #[serde(default)]
    pub expect_revoke_reverse_ops: Option<Vec<Op>>,
    /// If true, revoke must list at least one NonReversible/Compensation entry.
    #[serde(default)]
    pub expect_revoke_non_reversible: bool,
    /// Call `revoke` a second time (expects error).
    #[serde(default)]
    pub revoke_again: bool,
    /// Mint a Recovery Cap (LAW §13) before submit / reverse.
    #[serde(default)]
    pub mint_recovery: Option<VectorRecoveryMint>,
    /// Seed a Compensation lineage entry (submit auto-class is Inverse vs NonReversible only).
    #[serde(default)]
    pub compensation_commit: Option<VectorCompensationCommit>,
    /// After `end_activity`, require a NonReversible listing (compensation failure).
    #[serde(default)]
    pub expect_end_non_reversible: bool,
    /// `inject` names/services into an Activity Context before submit (LAW §8).
    #[serde(default)]
    pub inject: Option<VectorContextNames>,
    /// `limit` an Activity Context before submit (LAW §8). Only narrows.
    #[serde(default)]
    pub limit: Option<VectorContextNames>,
    /// `isolate` this Activity's Context slice before submit (LAW §8).
    #[serde(default)]
    pub isolate: Option<String>,
}

/// Vector fixture for [`VectorCase::inject`] / [`VectorCase::limit`] (LAW §8).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorContextNames {
    /// Activity whose Context is mediated.
    pub activity_id: String,
    /// Name/service tokens (`kv:greeting`, `kv`, `log`).
    pub names: Vec<String>,
}

/// Vector fixture for [`VectorCase::mint_recovery`] (LAW §13).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorRecoveryMint {
    /// Recovery Cap id.
    pub id: String,
    /// Declared compensation action.
    pub action: String,
    /// Once-Cap flag.
    #[serde(default)]
    pub once: bool,
    /// Optional expiry (unix seconds).
    #[serde(default)]
    pub not_after: Option<u64>,
    /// Bind to this Activity's reverse plan.
    #[serde(default)]
    pub for_activity: Option<String>,
    /// Bind to this lineage entry id.
    #[serde(default)]
    pub for_lineage: Option<String>,
    /// Sealed compensation args (ordinary Intent args under the Recovery Cap).
    #[serde(default)]
    pub sealed: BTreeMap<String, Value>,
}

/// Seed [`crate::ReverseClass::Compensation`] lineage for reverse (LAW §13).
///
/// Submit never auto-classifies Compensation; vectors commit this class explicitly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorCompensationCommit {
    /// Cap id that authorized the original cause.
    pub cap_id: String,
    /// Activity the cause belongs to.
    pub activity_id: String,
    /// Original action.
    pub action: String,
    /// Authorized Ops snapshot.
    #[serde(default)]
    pub ops: Vec<Op>,
}

/// Load a single vector JSON file.
pub fn load_vector_file(path: impl AsRef<Path>) -> ContractResult<VectorCase> {
    let data = std::fs::read_to_string(path)?;
    let v: VectorCase = serde_json::from_str(&data)?;
    if v.id.is_empty() || v.family.is_empty() {
        return Err(ContractError::VectorInvalid(
            "id and family required".into(),
        ));
    }
    Ok(v)
}

/// Load every `*.json` vector in `dir`, sorted by path.
pub fn load_vector_dir(dir: impl AsRef<Path>) -> ContractResult<Vec<(PathBuf, VectorCase)>> {
    let mut out = Vec::new();
    let mut paths: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("json"))
        .collect();
    paths.sort();
    for path in paths {
        let case = load_vector_file(&path)?;
        out.push((path, case));
    }
    Ok(out)
}

/// Check a Host Result against a vector case expectations.
pub fn check_result(case: &VectorCase, result: &ResultMsg) -> ContractResult<()> {
    let kind = match result.kind {
        crate::ResultKind::Ok => "ok",
        crate::ResultKind::AuthorityRefusal => "authority_refusal",
        crate::ResultKind::DispatchError => "dispatch_error",
    };
    if kind != case.expect_kind {
        return Err(ContractError::VectorInvalid(format!(
            "case {}: kind want {} got {}",
            case.id, case.expect_kind, kind
        )));
    }
    if case.expect_ops_empty && !result.ops.is_empty() {
        return Err(ContractError::VectorInvalid(format!(
            "case {}: expected empty ops, got {}",
            case.id,
            result.ops.len()
        )));
    }
    if let Some(ref expected) = case.expect_ops {
        if &result.ops != expected {
            return Err(ContractError::VectorInvalid(format!(
                "case {}: ops mismatch",
                case.id
            )));
        }
    }
    // Hard law: authority_refusal must be effect-free.
    if matches!(result.kind, crate::ResultKind::AuthorityRefusal) && !result.ops.is_empty() {
        return Err(ContractError::VectorInvalid(format!(
            "case {}: authority_refusal carried ops",
            case.id
        )));
    }
    if let Some(ref expected) = case.expect_lowered_ops {
        let got: Vec<Op> = result
            .ops
            .iter()
            .filter_map(crate::lower_to_baseline)
            .collect();
        if &got != expected {
            return Err(ContractError::VectorInvalid(format!(
                "case {}: lowered ops mismatch",
                case.id
            )));
        }
    }
    Ok(())
}
