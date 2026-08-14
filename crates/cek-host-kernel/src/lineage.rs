//! Lineage store (in-memory reference).
//!
//! ## Reverse preference (mature rule)
//!
//! 1. If `landed_ops` was annotated from a Peer receipt → reverse **landed**.
//! 2. Else reverse **authorized_ops** / inverse plan recorded at commit.
//! 3. `NonReversible` entries are listed, never faked as clean undo.
//!
//! Implements [`crate::LineageBackend`] so a durable backend can replace this.

use cek_contract::{LineageEntry, Op, ReverseClass};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use crate::{HostError, HostResult, LineageBackend};

/// Host lineage memory.
#[derive(Debug)]
pub struct LineageStore {
    seq: AtomicU64,
    by_activity: Mutex<HashMap<String, Vec<String>>>,
    by_id: Mutex<HashMap<String, LineageEntry>>,
    ended_activities: Mutex<HashSet<String>>,
}

impl Default for LineageStore {
    fn default() -> Self {
        Self {
            seq: AtomicU64::new(1),
            by_activity: Mutex::new(HashMap::new()),
            by_id: Mutex::new(HashMap::new()),
            ended_activities: Mutex::new(HashSet::new()),
        }
    }
}

impl LineageStore {
    /// Empty store.
    pub fn new() -> Self {
        Self::default()
    }

    fn next_id(&self) -> String {
        format!("lin-{}", self.seq.fetch_add(1, Ordering::Relaxed))
    }
}

impl LineageBackend for LineageStore {
    fn mark_ended(&self, activity_id: &str) -> HostResult<()> {
        let mut g = self
            .ended_activities
            .lock()
            .map_err(|_| HostError::Lineage("lock".into()))?;
        if !g.insert(activity_id.to_string()) {
            return Err(HostError::Lineage(format!(
                "activity already ended: {activity_id}"
            )));
        }
        Ok(())
    }

    fn is_ended(&self, activity_id: &str) -> bool {
        self.ended_activities
            .lock()
            .map(|g| g.contains(activity_id))
            .unwrap_or(false)
    }

    fn commit(
        &self,
        cap_id: &str,
        activity_id: Option<&str>,
        action: &str,
        authorized_ops: Vec<Op>,
        reverse_class: ReverseClass,
        inverse_ops: Vec<Op>,
    ) -> HostResult<LineageEntry> {
        // Fail closed: never attach a cause to an ended Activity.
        if let Some(aid) = activity_id {
            if self.is_ended(aid) {
                return Err(HostError::Lineage(format!(
                    "cannot commit to ended activity: {aid}"
                )));
            }
        }
        let entry = LineageEntry {
            id: self.next_id(),
            cap_id: cap_id.to_string(),
            activity_id: activity_id.map(|s| s.to_string()),
            action: action.to_string(),
            authorized_ops,
            reverse_class,
            inverse_ops,
            landed_ops: Vec::new(),
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
            by_act
                .entry(aid.to_string())
                .or_default()
                .push(entry.id.clone());
        }
        Ok(entry)
    }

    fn annotate_landed(&self, entry_id: &str, landed: Vec<Op>) -> HostResult<()> {
        let mut by_id = self
            .by_id
            .lock()
            .map_err(|_| HostError::Lineage("lock".into()))?;
        let e = by_id
            .get_mut(entry_id)
            .ok_or_else(|| HostError::Lineage(format!("unknown entry {entry_id}")))?;
        e.landed_ops = landed;
        Ok(())
    }

    fn annotate_landed_latest_for_activity(
        &self,
        activity_id: &str,
        landed: Vec<Op>,
    ) -> HostResult<()> {
        let ids = {
            let by_act = self
                .by_activity
                .lock()
                .map_err(|_| HostError::Lineage("lock".into()))?;
            by_act.get(activity_id).cloned().unwrap_or_default()
        };
        let Some(last) = ids.last() else {
            return Err(HostError::Lineage(format!(
                "no lineage for activity {activity_id}"
            )));
        };
        self.annotate_landed(last, landed)
    }

    fn for_activity(&self, activity_id: &str) -> HostResult<Vec<LineageEntry>> {
        let ids = {
            let g = self
                .by_activity
                .lock()
                .map_err(|_| HostError::Lineage("lock".into()))?;
            g.get(activity_id).cloned().unwrap_or_default()
        };
        let by_id = self
            .by_id
            .lock()
            .map_err(|_| HostError::Lineage("lock".into()))?;
        Ok(ids
            .into_iter()
            .filter_map(|id| by_id.get(&id).cloned())
            .collect())
    }
}

/// Outcome of reversing an Activity.
#[derive(Debug, Clone)]
pub struct ReverseOutcome {
    /// Ops to apply for undo (inverse / restore).
    pub ops: Vec<Op>,
    /// Entries marked non-reversible.
    pub non_reversible: Vec<String>,
    /// Whether any entry used landed set (receipt-informed).
    pub used_landed: bool,
}
