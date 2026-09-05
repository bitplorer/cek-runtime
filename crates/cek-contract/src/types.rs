//! Core CEK wire / in-process types.
//!
//! Field names match the conceptual charter. Encoding is JSON-friendly;
//! CBOR or other codecs may map 1:1 later without renaming concepts.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Permission ticket — sole authority object at the kernel boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cap {
    /// Opaque Cap identifier (Host-assigned).
    pub id: String,
    /// Action this Cap permits (exact match required on submit).
    pub action: String,
    /// Optional content digest / bind of sealed arguments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sealed_args_bind: Option<String>,
    /// Unix seconds; Cap is invalid at or after this instant (Host clock).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub not_after: Option<u64>,
    /// If true, Cap may be consumed at most once.
    #[serde(default)]
    pub once: bool,
    /// Optional subject bind (who may use the Cap).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    /// Narrowing scopes; never widen authority.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scopes: Vec<String>,
    /// Optional Host-policy MAC (`cek1:<hmac>`). Not law; verify is Host policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sig: Option<String>,
    /// Optional law-generation tag. Unset = current (legacy). Host dual-speak window.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub law_generation: Option<String>,
}

/// Sealed ask under a Cap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Intent {
    /// Action name (must match Cap.action).
    pub action: String,
    /// Caller-supplied arguments (open + conceptually sealed parts).
    #[serde(default)]
    pub args: BTreeMap<String, serde_json::Value>,
    /// Cap under which this Intent is submitted.
    pub cap: Cap,
    /// Correlation only — never authority.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace: Option<String>,
    /// Optional idempotency key for retries of the same logical ask.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
    /// Optional Activity this Intent belongs to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activity_id: Option<String>,
}

/// Single ordered effect as pure data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Op {
    /// Namespace (e.g. `kv`, `log`, `ui.dom`). Domain ns is the pack.
    pub ns: String,
    /// Op name within namespace — single token (e.g. `set`, `append`, `morph`).
    pub name: String,
    /// Data-only payload.
    #[serde(default)]
    pub payload: serde_json::Value,
}

impl Op {
    /// Fully-qualified name `ns.name`.
    pub fn fq(&self) -> String {
        format!("{}.{}", self.ns, self.name)
    }
}

/// How a Result concluded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultKind {
    /// Cap verified; Ops may be present.
    Ok,
    /// Cap path failed; **must** have empty mutate Ops.
    AuthorityRefusal,
    /// Cap ok but dispatch/handler failed.
    DispatchError,
}

/// Host answer to an Intent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultMsg {
    /// Outcome kind.
    pub kind: ResultKind,
    /// Authorized Ops (empty on authority_refusal).
    #[serde(default)]
    pub ops: Vec<Op>,
    /// Human/machine error detail.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Optional stable digest of this Result for idempotent replay.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
}

impl ResultMsg {
    /// Fail-closed Cap refusal with zero Ops.
    pub fn authority_refusal(msg: impl Into<String>) -> Self {
        Self {
            kind: ResultKind::AuthorityRefusal,
            ops: Vec::new(),
            error: Some(msg.into()),
            digest: None,
        }
    }

    /// Successful authorization with Ops.
    pub fn ok(ops: Vec<Op>) -> Self {
        Self {
            kind: ResultKind::Ok,
            ops,
            error: None,
            digest: None,
        }
    }

    /// Dispatch-time failure after Cap verify.
    pub fn dispatch_error(msg: impl Into<String>) -> Self {
        Self {
            kind: ResultKind::DispatchError,
            ops: Vec::new(),
            error: Some(msg.into()),
            digest: None,
        }
    }

    /// True if this Result must not mutate the world via Ops.
    pub fn is_effect_free(&self) -> bool {
        matches!(self.kind, ResultKind::AuthorityRefusal) || self.ops.is_empty()
    }
}

/// Reverse plan class attached at lineage commit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReverseClass {
    /// Inverse Ops can undo.
    Inverse,
    /// Compensation under recovery Cap.
    Compensation,
    /// Cannot cleanly undo; mark honestly.
    NonReversible,
}

/// Lineage cause entry (Host-owned).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineageEntry {
    /// Entry id.
    pub id: String,
    /// Cap id that authorized this cause.
    pub cap_id: String,
    /// Optional Activity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activity_id: Option<String>,
    /// Optional Intent.trace — correlation only, never authority (LAW §10).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace: Option<String>,
    /// Action from Intent.
    pub action: String,
    /// Authorized Ops snapshot.
    pub authorized_ops: Vec<Op>,
    /// Reverse class for this cause.
    pub reverse_class: ReverseClass,
    /// Optional inverse Ops if known at commit.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inverse_ops: Vec<Op>,
    /// Landed Ops from Peer receipt (annotated after apply). Empty = unknown.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub landed_ops: Vec<Op>,
}

/// Peer report of what actually applied — **not** a Cap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    /// Ops that landed successfully.
    #[serde(default)]
    pub landed: Vec<Op>,
    /// Ops that failed to apply.
    #[serde(default)]
    pub failed: Vec<Op>,
}

/// What a Peer claims it can apply.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    /// Profile name (e.g. `baseline`, `production-v1`).
    pub name: String,
    /// Fully-qualified Op names this Peer applies.
    pub apply_set: Vec<String>,
    /// Policy for unknown Ops.
    pub unknown_op_policy: UnknownOpPolicy,
}

/// Peer behavior for Ops outside apply_set.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnknownOpPolicy {
    /// Skip and record failed.
    #[default]
    Skip,
    /// Fail the whole apply batch.
    FailBatch,
}

/// Mediated visible world of an Activity (LAW §8 / CORE 07).
///
/// Not ambient authority, not a Cap substitute, not a global bag of power.
/// Host `inject` / `limit` / `isolate` only **narrow** (axiom A8).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Context {
    /// Activity this Context is bound to.
    pub activity_id: String,
    /// Names/services the Activity requires (`inject`). Undeclared access fails closed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub injected: Vec<String>,
    /// Restriction tokens (`limit`). Only narrow what may be seen or done.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub limits: Vec<String>,
    /// When true, this slice is `isolate`d — names/services do not leak across Activities.
    #[serde(default)]
    pub isolated: bool,
}

/// Process handshake document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    /// Law generation id (current).
    pub law_generation: String,
    /// Generations this Host will accept (dual-speak window). Unset → `[law_generation]`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub accepted_generations: Vec<String>,
    /// Supported profiles.
    pub profiles: Vec<String>,
    /// Fail-closed facts.
    #[serde(default)]
    pub fail_closed: FailClosed,
}

/// Declared fail-closed behaviors (handshake; Host always enforces these).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailClosed {
    /// If once-store is down, refuse (do not skip once).
    #[serde(default = "default_true")]
    pub once_store_down: bool,
    /// If idempotency store is down, refuse.
    #[serde(default = "default_true")]
    pub idem_store_down: bool,
    /// Sealed-args bind is enforced when present.
    #[serde(default = "default_true")]
    pub sealed_args: bool,
    /// Non-empty Cap.scopes is an allow-list.
    #[serde(default = "default_true")]
    pub scopes: bool,
    /// Cap HMAC is required when this Host was constructed with a key.
    #[serde(default)]
    pub cap_signatures: bool,
}

impl Default for FailClosed {
    fn default() -> Self {
        Self {
            once_store_down: true,
            idem_store_down: true,
            sealed_args: true,
            scopes: true,
            cap_signatures: false,
        }
    }
}

fn default_true() -> bool {
    true
}
