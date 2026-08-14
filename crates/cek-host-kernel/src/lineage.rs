//! Lineage store (in-memory reference).

use cek_contract::{LineageEntry, Op, ReverseClass};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use crate::{HostError, HostResult};

static LINEAGE_SEQ: AtomicU64 = AtomicU64::new(1);

fn next_id() -> String {
    format!("lin-{}", LINEAGE_SEQ.fetch_add(1, Ordering::Relaxed))
}

/// Host lineage memory.
#[derive(Debug, Default)]
pub struct LineageStore {
    by_activity: Mutex<HashMap<String, Vec<LineageEntry>>>,
    by_id: Mutex<HashMap<String, LineageEntry>>,
}

impl LineageStore {
    /// Empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a cause under optional Activity.
    pub fn commit(
        &self,
        cap_id: &str,
        activity_id: Option<&str>,
        action: &str,
        authorized_ops: Vec<Op>,
        reverse_class: ReverseClass,
        inverse_ops: Vec<Op>,
    ) -> HostResult<LineageEntry> {
        let entry = LineageEntry {
            id: next_id(),
            cap_id: cap_id.to_string(),
            activity_id: activity_id.map(|s| s.to_string()),
            action: action.to_string(),
            authorized_ops,
            reverse_class,
            inverse_ops,
        };
        {
            let mut by_id = self
                .by_id
                .lock()
                .map_err(|_| HostError::Lineage("lock".into()))?;
            by_id.insert(entry.id.clone(), entry.clone());
        }
        if let Some(aid) = activity_id {
            let mut by_act = self
                .by_activity
                .lock()
                .map_err(|_| HostError::Lineage("lock".into()))?;
            by_act.entry(aid.to_string()).or_default().push(entry.clone());
        }
        Ok(entry)
    }

    /// Entries for an Activity (authorized set order).
    pub fn for_activity(&self, activity_id: &str) -> HostResult<Vec<LineageEntry>> {
        let g = self
            .by_activity
            .lock()
            .map_err(|_| HostError::Lineage("lock".into()))?;
        Ok(g.get(activity_id).cloned().unwrap_or_default())
    }
}

/// Outcome of reversing an Activity.
#[derive(Debug, Clone)]
pub struct ReverseOutcome {
    /// Ops to apply for undo (inverse / restore).
    pub ops: Vec<Op>,
    /// Entries marked non-reversible.
    pub non_reversible: Vec<String>,
}
