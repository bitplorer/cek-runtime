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

use crate::store::persistable_trace;
use crate::{HostError, HostResult, LineageBackend};

/// Host lineage memory.
#[derive(Debug)]
pub struct LineageStore {
    seq: AtomicU64,
    by_activity: Mutex<HashMap<String, Vec<String>>>,
    by_cap: Mutex<HashMap<String, Vec<String>>>,
    by_trace: Mutex<HashMap<String, Vec<String>>>,
    by_id: Mutex<HashMap<String, LineageEntry>>,
    ended_activities: Mutex<HashSet<String>>,
    revoked: Mutex<HashSet<String>>,
}

impl Default for LineageStore {
    fn default() -> Self {
        Self {
            seq: AtomicU64::new(1),
            by_activity: Mutex::new(HashMap::new()),
            by_cap: Mutex::new(HashMap::new()),
            by_trace: Mutex::new(HashMap::new()),
            by_id: Mutex::new(HashMap::new()),
            ended_activities: Mutex::new(HashSet::new()),
            revoked: Mutex::new(HashSet::new()),
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
        trace: Option<&str>,
    ) -> HostResult<LineageEntry> {
        // Fail closed: never attach a cause to an ended Activity or revoked Cap.
        // Trace is never a gate here (LAW §10).
        if let Some(aid) = activity_id {
            if self.is_ended(aid) {
                return Err(HostError::Lineage(format!(
                    "cannot commit to ended activity: {aid}"
                )));
            }
        }
        if self.is_revoked(cap_id) {
            return Err(HostError::Lineage(format!(
                "cannot commit under revoked Cap: {cap_id}"
            )));
        }
        let trace = persistable_trace(trace);
        let entry = LineageEntry {
            id: self.next_id(),
            cap_id: cap_id.to_string(),
            activity_id: activity_id.map(|s| s.to_string()),
            trace: trace.clone(),
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
        {
            let mut by_cap = self
                .by_cap
                .lock()
                .map_err(|_| HostError::Lineage("lock".into()))?;
            by_cap
                .entry(cap_id.to_string())
                .or_default()
                .push(entry.id.clone());
        }
        if let Some(tr) = trace {
            let mut by_tr = self
                .by_trace
                .lock()
                .map_err(|_| HostError::Lineage("lock".into()))?;
            by_tr.entry(tr).or_default().push(entry.id.clone());
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
        self.entries_for_ids(ids)
    }

    fn for_cap(&self, cap_id: &str) -> HostResult<Vec<LineageEntry>> {
        let ids = {
            let g = self
                .by_cap
                .lock()
                .map_err(|_| HostError::Lineage("lock".into()))?;
            g.get(cap_id).cloned().unwrap_or_default()
        };
        self.entries_for_ids(ids)
    }

    fn for_trace(&self, trace: &str) -> HostResult<Vec<LineageEntry>> {
        let key = persistable_trace(Some(trace));
        let Some(key) = key else {
            return Ok(Vec::new());
        };
        let ids = {
            let g = self
                .by_trace
                .lock()
                .map_err(|_| HostError::Lineage("lock".into()))?;
            g.get(&key).cloned().unwrap_or_default()
        };
        self.entries_for_ids(ids)
    }

    fn mark_revoked(&self, cap_id: &str) -> HostResult<()> {
        let mut g = self
            .revoked
            .lock()
            .map_err(|_| HostError::Lineage("lock".into()))?;
        if !g.insert(cap_id.to_string()) {
            return Err(HostError::Lineage(format!("Cap already revoked: {cap_id}")));
        }
        Ok(())
    }

    fn is_revoked(&self, cap_id: &str) -> bool {
        self.revoked
            .lock()
            .map(|g| g.contains(cap_id))
            .unwrap_or(false)
    }

    fn ensure_not_revoked(&self, cap_id: &str) -> HostResult<()> {
        let g = self
            .revoked
            .lock()
            .map_err(|_| HostError::Lineage("lock".into()))?;
        if g.contains(cap_id) {
            return Err(HostError::Authority(format!("Cap revoked: {cap_id}")));
        }
        Ok(())
    }
}

impl LineageStore {
    fn entries_for_ids(&self, ids: Vec<String>) -> HostResult<Vec<LineageEntry>> {
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

/// Outcome of reversing an Activity or a Cap (LAW §9).
#[derive(Debug, Clone)]
pub struct ReverseOutcome {
    /// Ops to apply for undo (inverse / restore / compensation Result.ops).
    pub ops: Vec<Op>,
    /// Entries marked non-reversible (including failed compensation).
    pub non_reversible: Vec<String>,
    /// Whether any entry used landed set (receipt-informed).
    pub used_landed: bool,
}
