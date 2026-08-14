//! Conformance vector loading (JSON fixtures).

use crate::{ContractError, ContractResult, Intent, ResultMsg};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// One executable conformance case.
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
    pub expect_ops: Option<Vec<crate::Op>>,
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
    Ok(())
}
