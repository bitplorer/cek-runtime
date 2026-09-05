//! Durable store **traits** for once / idempotency / lineage.
//!
//! The in-memory types ([`crate::OnceStore`], [`crate::IdemStore`],
//! [`crate::LineageStore`]) are the reference backends. File-backed
//! implementations live in [`crate::durable`]. Host talks only to these
//! traits so a durable backend cannot change authority law.
//!
//! ## Contracts (must not regress)
//!
//! | Trait | Fail closed | Sequencing |
//! |-------|-------------|------------|
//! | [`OnceBackend`] | store down → refuse | `ensure_available` **before** dispatch; `commit` **only after** successful dispatch |
//! | [`IdemBackend`] | store down → refuse | same key + same digest → replay; same key + different digest → refuse |
//! | [`LineageBackend`] | store down → error | no commit after Activity ended or Cap revoked; landed annotation is optional |

use crate::{HostResult, IdemOutcome};
use cek_contract::{LineageEntry, Op, ResultMsg, ReverseClass};

/// Once-Cap consume backend.
///
/// Implementations MUST:
/// - treat a down/unavailable store as [`crate::HostError::OnceStoreDown`]
///   (never skip the once check);
/// - **not** mark consumed in [`OnceBackend::ensure_available`];
/// - mark consumed only in [`OnceBackend::commit`];
/// - treat a second `commit` of the same once-Cap as authority failure;
/// - treat `once == false` as a no-op (always available, never recorded).
pub trait OnceBackend: Send + Sync {
    /// Refuse if this once-Cap is already consumed. Does **not** record.
    fn ensure_available(&self, cap_id: &str, once: bool) -> HostResult<()>;

    /// Record consumption after a successful dispatch. No-op when `once` is false.
    fn commit(&self, cap_id: &str, once: bool) -> HostResult<()>;

    /// Whether `cap_id` is marked consumed. `false` if the store cannot be read.
    fn is_consumed(&self, cap_id: &str) -> bool;
}

/// Idempotency bind backend.
///
/// Implementations MUST:
/// - treat a down store as [`crate::HostError::IdemStoreDown`];
/// - return the cached Result on same key + same digest;
/// - refuse (authority) on same key + different digest.
pub trait IdemBackend: Send + Sync {
    /// Lookup a prior Result for `key`.
    fn get(&self, key: &str) -> HostResult<Option<ResultMsg>>;

    /// Record or detect conflict for `key` bound to `digest`.
    fn put_or_check(&self, key: &str, digest: &str, result: &ResultMsg) -> HostResult<IdemOutcome>;
}

/// Lineage + reverse-annotation backend.
///
/// Implementations MUST:
/// - reject a second [`LineageBackend::mark_ended`] for the same Activity;
/// - reject [`LineageBackend::commit`] onto an already-ended Activity;
/// - reject a second [`LineageBackend::mark_revoked`] for the same Cap;
/// - reject [`LineageBackend::commit`] under a revoked Cap;
/// - persist `landed_ops` from receipts so reverse can prefer landed;
/// - index causes by Cap so [`LineageBackend::for_cap`] is Cap-scoped (LAW §9);
/// - persist optional `Intent.trace` and index by it so [`LineageBackend::for_trace`]
///   groups related Intents (LAW §10). Trace is correlation only — never authority,
///   never reverse-by-trace, never a Cap substitute.
pub trait LineageBackend: Send + Sync {
    /// Mark Activity ended. Second end is an error.
    fn mark_ended(&self, activity_id: &str) -> HostResult<()>;

    /// True if Activity was ended. `false` if the store cannot be read.
    fn is_ended(&self, activity_id: &str) -> bool;

    /// Record an authorized cause. Must not attach to an ended Activity.
    ///
    /// `trace` is optional correlation (LAW §10). Empty / whitespace is stored as
    /// absent (no-trace is OK). Implementations must not treat `trace` as
    /// permission, undo, or a Cap substitute.
    fn commit(
        &self,
        cap_id: &str,
        activity_id: Option<&str>,
        action: &str,
        authorized_ops: Vec<Op>,
        reverse_class: ReverseClass,
        inverse_ops: Vec<Op>,
        trace: Option<&str>,
    ) -> HostResult<LineageEntry>;

    /// Annotate an entry with Peer landed Ops (from a receipt).
    fn annotate_landed(&self, entry_id: &str, landed: Vec<Op>) -> HostResult<()>;

    /// Annotate the latest entry for an Activity (in-process receipt path).
    fn annotate_landed_latest_for_activity(
        &self,
        activity_id: &str,
        landed: Vec<Op>,
    ) -> HostResult<()>;

    /// Entries for an Activity in commit order.
    fn for_activity(&self, activity_id: &str) -> HostResult<Vec<LineageEntry>>;

    /// Entries for a Cap in commit order.
    fn for_cap(&self, cap_id: &str) -> HostResult<Vec<LineageEntry>>;

    /// Entries that share a trace, in commit order (LAW §10 grouping).
    ///
    /// Query only — never grants authority, never reverses, never revives a Cap.
    /// Unknown / empty / no-trace returns an empty list (not an error).
    fn for_trace(&self, trace: &str) -> HostResult<Vec<LineageEntry>>;

    /// Mark Cap revoked (LAW §5 Active→Revoked). Second revoke is an error.
    fn mark_revoked(&self, cap_id: &str) -> HostResult<()>;

    /// True if Cap was revoked. `false` if the store cannot be read.
    fn is_revoked(&self, cap_id: &str) -> bool;

    /// Refuse if this Cap is revoked. Store down is an error (fail closed).
    fn ensure_not_revoked(&self, cap_id: &str) -> HostResult<()>;
}

/// Persistable correlation id: present and non-empty after trim. Empty is no-trace.
pub(crate) fn persistable_trace(trace: Option<&str>) -> Option<String> {
    trace
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{IdemStore, LineageStore, OnceStore};
    use cek_contract::{baseline, ResultMsg, ReverseClass};
    use serde_json::json;

    fn once_contract(b: &dyn OnceBackend) {
        b.ensure_available("c", true).unwrap();
        assert!(!b.is_consumed("c"), "ensure must not burn");
        b.commit("c", true).unwrap();
        assert!(b.is_consumed("c"));
        assert!(b.ensure_available("c", true).is_err());
        assert!(b.commit("c", true).is_err(), "second commit refuses");
        b.ensure_available("n", false).unwrap();
        b.commit("n", false).unwrap();
        assert!(!b.is_consumed("n"), "non-once is never recorded");
    }

    fn idem_contract(b: &dyn IdemBackend) {
        let r = ResultMsg::ok(vec![baseline::kv_set("k", json!(1))]);
        assert!(b.get("ik").unwrap().is_none());
        match b.put_or_check("ik", "cek1:aaa", &r).unwrap() {
            crate::IdemOutcome::Recorded => {}
            other => panic!("expected Recorded, got {other:?}"),
        }
        match b.put_or_check("ik", "cek1:aaa", &r).unwrap() {
            crate::IdemOutcome::ReplaySame { result } => {
                assert_eq!(result.ops, r.ops);
            }
            other => panic!("expected ReplaySame, got {other:?}"),
        }
        assert!(b.put_or_check("ik", "cek1:bbb", &r).is_err());
    }

    fn lineage_contract(b: &dyn LineageBackend) {
        let ops = vec![baseline::kv_set("k", json!(1))];
        let inv = vec![baseline::kv_delete("k")];
        let e = b
            .commit(
                "cap",
                Some("act"),
                "kv.write",
                ops.clone(),
                ReverseClass::Inverse,
                inv.clone(),
                Some("shared-trace"),
            )
            .unwrap();
        assert!(!e.id.is_empty());
        assert_eq!(e.trace.as_deref(), Some("shared-trace"));
        b.annotate_landed_latest_for_activity("act", ops.clone())
            .unwrap();
        let got = b.for_activity("act").unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].landed_ops, ops);
        let grouped = b.for_trace("shared-trace").unwrap();
        assert_eq!(grouped.len(), 1);
        assert_eq!(grouped[0].cap_id, "cap");
        assert!(b.for_trace("other-trace").unwrap().is_empty());
        assert!(b.for_trace("").unwrap().is_empty());
        b.mark_ended("act").unwrap();
        assert!(b.is_ended("act"));
        assert!(b.mark_ended("act").is_err());
        assert!(b
            .commit(
                "cap",
                Some("act"),
                "kv.write",
                ops.clone(),
                ReverseClass::Inverse,
                inv.clone(),
                Some("shared-trace"),
            )
            .is_err());
        let by_cap = b.for_cap("cap").unwrap();
        assert_eq!(by_cap.len(), 1);
        assert_eq!(by_cap[0].cap_id, "cap");
        b.mark_revoked("cap").unwrap();
        assert!(b.is_revoked("cap"));
        assert!(b.ensure_not_revoked("cap").is_err());
        assert!(b.mark_revoked("cap").is_err());
        assert!(b
            .commit(
                "cap",
                Some("act-2"),
                "kv.write",
                ops,
                ReverseClass::Inverse,
                inv,
                None,
            )
            .is_err());
        b.ensure_not_revoked("other").unwrap();
    }

    #[test]
    fn persistable_trace_drops_empty() {
        assert_eq!(persistable_trace(None), None);
        assert_eq!(persistable_trace(Some("")), None);
        assert_eq!(persistable_trace(Some("   ")), None);
        assert_eq!(persistable_trace(Some("shared")), Some("shared".into()));
    }

    #[test]
    fn memory_once_satisfies_contract() {
        once_contract(&OnceStore::new());
    }

    #[test]
    fn memory_idem_satisfies_contract() {
        idem_contract(&IdemStore::new());
    }

    #[test]
    fn memory_lineage_satisfies_contract() {
        lineage_contract(&LineageStore::new());
    }
}
