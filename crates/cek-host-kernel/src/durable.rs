//! File-backed durable backends for once / idempotency / lineage.
//!
//! These implement the same traits as the in-memory stores. Persistence is
//! JSON + atomic rename. I/O failure is **fail closed** (never skip once).
//!
//! Multi-process locking and a real DB are still out of scope (see HARDENING).
//! One store instance per directory is the supported use.

use crate::{HostError, HostResult, IdemBackend, IdemOutcome, LineageBackend, OnceBackend};
use cek_contract::{LineageEntry, Op, ResultMsg, ReverseClass};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

fn persist(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let tmp = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    Ok(())
}

fn load<T: for<'de> Deserialize<'de> + Default>(path: &Path) -> Result<T, String> {
    if !path.exists() {
        return Ok(T::default());
    }
    let data = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    if data.trim().is_empty() {
        return Ok(T::default());
    }
    serde_json::from_str(&data).map_err(|e| e.to_string())
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct OnceSnap {
    consumed: BTreeSet<String>,
}

/// File-backed once-Cap store (`once.json` in `dir`).
pub struct FileOnceStore {
    path: PathBuf,
    inner: Mutex<OnceSnap>,
}

impl FileOnceStore {
    /// Open or create `dir/once.json`.
    pub fn open(dir: impl AsRef<Path>) -> HostResult<Self> {
        let dir = dir.as_ref();
        std::fs::create_dir_all(dir).map_err(|_| HostError::OnceStoreDown)?;
        let path = dir.join("once.json");
        let snap: OnceSnap = load(&path).map_err(|_| HostError::OnceStoreDown)?;
        Ok(Self {
            path,
            inner: Mutex::new(snap),
        })
    }

    fn flush(&self, snap: &OnceSnap) -> HostResult<()> {
        persist(&self.path, snap).map_err(|_| HostError::OnceStoreDown)
    }
}

impl OnceBackend for FileOnceStore {
    fn ensure_available(&self, cap_id: &str, once: bool) -> HostResult<()> {
        if !once {
            return Ok(());
        }
        let g = self.inner.lock().map_err(|_| HostError::OnceStoreDown)?;
        if g.consumed.contains(cap_id) {
            return Err(HostError::Authority(format!(
                "once Cap already consumed: {cap_id}"
            )));
        }
        Ok(())
    }

    fn commit(&self, cap_id: &str, once: bool) -> HostResult<()> {
        if !once {
            return Ok(());
        }
        let mut g = self.inner.lock().map_err(|_| HostError::OnceStoreDown)?;
        if !g.consumed.insert(cap_id.to_string()) {
            return Err(HostError::Authority(format!(
                "once Cap already consumed: {cap_id}"
            )));
        }
        self.flush(&g)?;
        Ok(())
    }

    fn is_consumed(&self, cap_id: &str) -> bool {
        self.inner
            .lock()
            .map(|g| g.consumed.contains(cap_id))
            .unwrap_or(false)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct IdemSnapEntry {
    digest: String,
    result: ResultMsg,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct IdemSnap {
    entries: BTreeMap<String, IdemSnapEntry>,
}

/// File-backed idempotency store (`idem.json` in `dir`).
pub struct FileIdemStore {
    path: PathBuf,
    inner: Mutex<IdemSnap>,
}

impl FileIdemStore {
    /// Open or create `dir/idem.json`.
    pub fn open(dir: impl AsRef<Path>) -> HostResult<Self> {
        let dir = dir.as_ref();
        std::fs::create_dir_all(dir).map_err(|_| HostError::IdemStoreDown)?;
        let path = dir.join("idem.json");
        let snap: IdemSnap = load(&path).map_err(|_| HostError::IdemStoreDown)?;
        Ok(Self {
            path,
            inner: Mutex::new(snap),
        })
    }

    fn flush(&self, snap: &IdemSnap) -> HostResult<()> {
        persist(&self.path, snap).map_err(|_| HostError::IdemStoreDown)
    }
}

impl IdemBackend for FileIdemStore {
    fn get(&self, key: &str) -> HostResult<Option<ResultMsg>> {
        let g = self.inner.lock().map_err(|_| HostError::IdemStoreDown)?;
        Ok(g.entries.get(key).map(|e| e.result.clone()))
    }

    fn put_or_check(&self, key: &str, digest: &str, result: &ResultMsg) -> HostResult<IdemOutcome> {
        let mut g = self.inner.lock().map_err(|_| HostError::IdemStoreDown)?;
        match g.entries.get(key) {
            None => {
                g.entries.insert(
                    key.to_string(),
                    IdemSnapEntry {
                        digest: digest.to_string(),
                        result: result.clone(),
                    },
                );
                self.flush(&g)?;
                Ok(IdemOutcome::Recorded)
            }
            Some(prev) if prev.digest == digest => Ok(IdemOutcome::ReplaySame {
                result: prev.result.clone(),
            }),
            Some(_) => Err(HostError::Authority(format!(
                "idempotency conflict for key `{key}`"
            ))),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct LineageSnap {
    seq: u64,
    by_id: BTreeMap<String, LineageEntry>,
    by_activity: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    by_cap: BTreeMap<String, Vec<String>>,
    ended: BTreeSet<String>,
    #[serde(default)]
    revoked: BTreeSet<String>,
}

/// File-backed lineage store (`lineage.json` in `dir`).
pub struct FileLineageStore {
    path: PathBuf,
    inner: Mutex<LineageSnap>,
}

impl FileLineageStore {
    /// Open or create `dir/lineage.json`.
    pub fn open(dir: impl AsRef<Path>) -> HostResult<Self> {
        let dir = dir.as_ref();
        std::fs::create_dir_all(dir).map_err(|e| HostError::Lineage(e.to_string()))?;
        let path = dir.join("lineage.json");
        let mut snap: LineageSnap = load(&path).map_err(HostError::Lineage)?;
        if snap.seq == 0 {
            snap.seq = 1;
        }
        if snap.by_cap.is_empty() && !snap.by_id.is_empty() {
            let mut ids: Vec<_> = snap.by_id.keys().cloned().collect();
            ids.sort_by_key(|id| {
                id.strip_prefix("lin-")
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(u64::MAX)
            });
            for id in ids {
                if let Some(e) = snap.by_id.get(&id) {
                    snap.by_cap.entry(e.cap_id.clone()).or_default().push(id);
                }
            }
        }
        Ok(Self {
            path,
            inner: Mutex::new(snap),
        })
    }

    fn flush(&self, snap: &LineageSnap) -> HostResult<()> {
        persist(&self.path, snap).map_err(HostError::Lineage)
    }
}

impl LineageBackend for FileLineageStore {
    fn mark_ended(&self, activity_id: &str) -> HostResult<()> {
        let mut g = self
            .inner
            .lock()
            .map_err(|_| HostError::Lineage("lock".into()))?;
        if !g.ended.insert(activity_id.to_string()) {
            return Err(HostError::Lineage(format!(
                "activity already ended: {activity_id}"
            )));
        }
        self.flush(&g)?;
        Ok(())
    }

    fn is_ended(&self, activity_id: &str) -> bool {
        self.inner
            .lock()
            .map(|g| g.ended.contains(activity_id))
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
        let mut g = self
            .inner
            .lock()
            .map_err(|_| HostError::Lineage("lock".into()))?;
        if let Some(aid) = activity_id {
            if g.ended.contains(aid) {
                return Err(HostError::Lineage(format!(
                    "cannot commit to ended activity: {aid}"
                )));
            }
        }
        if g.revoked.contains(cap_id) {
            return Err(HostError::Lineage(format!(
                "cannot commit under revoked Cap: {cap_id}"
            )));
        }
        let id = format!("lin-{}", g.seq);
        g.seq = g.seq.saturating_add(1);
        let entry = LineageEntry {
            id: id.clone(),
            cap_id: cap_id.to_string(),
            activity_id: activity_id.map(|s| s.to_string()),
            action: action.to_string(),
            authorized_ops,
            reverse_class,
            inverse_ops,
            landed_ops: Vec::new(),
        };
        g.by_id.insert(id.clone(), entry.clone());
        if let Some(aid) = activity_id {
            g.by_activity
                .entry(aid.to_string())
                .or_default()
                .push(id.clone());
        }
        g.by_cap.entry(cap_id.to_string()).or_default().push(id);
        self.flush(&g)?;
        Ok(entry)
    }

    fn annotate_landed(&self, entry_id: &str, landed: Vec<Op>) -> HostResult<()> {
        let mut g = self
            .inner
            .lock()
            .map_err(|_| HostError::Lineage("lock".into()))?;
        let e = g
            .by_id
            .get_mut(entry_id)
            .ok_or_else(|| HostError::Lineage(format!("unknown entry {entry_id}")))?;
        e.landed_ops = landed;
        self.flush(&g)?;
        Ok(())
    }

    fn annotate_landed_latest_for_activity(
        &self,
        activity_id: &str,
        landed: Vec<Op>,
    ) -> HostResult<()> {
        let last = {
            let g = self
                .inner
                .lock()
                .map_err(|_| HostError::Lineage("lock".into()))?;
            g.by_activity
                .get(activity_id)
                .and_then(|ids| ids.last().cloned())
                .ok_or_else(|| {
                    HostError::Lineage(format!("no lineage for activity {activity_id}"))
                })?
        };
        self.annotate_landed(&last, landed)
    }

    fn for_activity(&self, activity_id: &str) -> HostResult<Vec<LineageEntry>> {
        let g = self
            .inner
            .lock()
            .map_err(|_| HostError::Lineage("lock".into()))?;
        let ids = g.by_activity.get(activity_id).cloned().unwrap_or_default();
        Ok(ids
            .into_iter()
            .filter_map(|id| g.by_id.get(&id).cloned())
            .collect())
    }

    fn for_cap(&self, cap_id: &str) -> HostResult<Vec<LineageEntry>> {
        let g = self
            .inner
            .lock()
            .map_err(|_| HostError::Lineage("lock".into()))?;
        let ids = g.by_cap.get(cap_id).cloned().unwrap_or_default();
        Ok(ids
            .into_iter()
            .filter_map(|id| g.by_id.get(&id).cloned())
            .collect())
    }

    fn mark_revoked(&self, cap_id: &str) -> HostResult<()> {
        let mut g = self
            .inner
            .lock()
            .map_err(|_| HostError::Lineage("lock".into()))?;
        if !g.revoked.insert(cap_id.to_string()) {
            return Err(HostError::Lineage(format!("Cap already revoked: {cap_id}")));
        }
        self.flush(&g)?;
        Ok(())
    }

    fn is_revoked(&self, cap_id: &str) -> bool {
        self.inner
            .lock()
            .map(|g| g.revoked.contains(cap_id))
            .unwrap_or(false)
    }

    fn ensure_not_revoked(&self, cap_id: &str) -> HostResult<()> {
        let g = self
            .inner
            .lock()
            .map_err(|_| HostError::Lineage("lock".into()))?;
        if g.revoked.contains(cap_id) {
            return Err(HostError::Authority(format!("Cap revoked: {cap_id}")));
        }
        Ok(())
    }
}

/// Open all three file-backed stores in one directory.
pub struct FileStores {
    /// Once-Cap consume store.
    pub once: FileOnceStore,
    /// Idempotency bind store.
    pub idem: FileIdemStore,
    /// Lineage + receipt store.
    pub lineage: FileLineageStore,
}

impl FileStores {
    /// Open or create `dir/{once,idem,lineage}.json`.
    pub fn open(dir: impl AsRef<Path>) -> HostResult<Self> {
        let dir = dir.as_ref();
        Ok(Self {
            once: FileOnceStore::open(dir)?,
            idem: FileIdemStore::open(dir)?,
            lineage: FileLineageStore::open(dir)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{IdemBackend, LineageBackend, OnceBackend};
    use cek_contract::{baseline, ResultMsg, ReverseClass};
    use serde_json::json;
    use std::sync::atomic::{AtomicU64, Ordering};

    static DIR_SEQ: AtomicU64 = AtomicU64::new(1);

    fn tmp_dir() -> PathBuf {
        let n = DIR_SEQ.fetch_add(1, Ordering::Relaxed);
        let p = std::env::temp_dir().join(format!("cek-durable-{}-{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn file_once_survives_reopen() {
        let dir = tmp_dir();
        {
            let s = FileOnceStore::open(&dir).unwrap();
            s.ensure_available("cap-1", true).unwrap();
            assert!(!s.is_consumed("cap-1"));
            s.commit("cap-1", true).unwrap();
            assert!(s.is_consumed("cap-1"));
        }
        let s2 = FileOnceStore::open(&dir).unwrap();
        assert!(s2.is_consumed("cap-1"));
        assert!(s2.ensure_available("cap-1", true).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn file_idem_survives_reopen() {
        let dir = tmp_dir();
        let r = ResultMsg::ok(vec![baseline::kv_set("k", json!(1))]);
        {
            let s = FileIdemStore::open(&dir).unwrap();
            match s.put_or_check("ik", "cek1:aaa", &r).unwrap() {
                IdemOutcome::Recorded => {}
                other => panic!("{other:?}"),
            }
        }
        let s2 = FileIdemStore::open(&dir).unwrap();
        match s2.put_or_check("ik", "cek1:aaa", &r).unwrap() {
            IdemOutcome::ReplaySame { result } => assert_eq!(result.ops, r.ops),
            other => panic!("{other:?}"),
        }
        assert!(s2.put_or_check("ik", "cek1:bbb", &r).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn file_lineage_survives_reopen_and_landed() {
        let dir = tmp_dir();
        let ops = vec![baseline::kv_set("k", json!(1))];
        let inv = vec![baseline::kv_delete("k")];
        {
            let s = FileLineageStore::open(&dir).unwrap();
            s.commit(
                "cap",
                Some("act"),
                "kv.write",
                ops.clone(),
                ReverseClass::Inverse,
                inv,
            )
            .unwrap();
            s.annotate_landed_latest_for_activity("act", ops.clone())
                .unwrap();
        }
        let s2 = FileLineageStore::open(&dir).unwrap();
        let got = s2.for_activity("act").unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].landed_ops, ops);
        s2.mark_ended("act").unwrap();
        drop(s2);
        let s3 = FileLineageStore::open(&dir).unwrap();
        assert!(s3.is_ended("act"));
        assert!(s3
            .commit(
                "cap",
                Some("act"),
                "kv.write",
                ops,
                ReverseClass::Inverse,
                vec![],
            )
            .is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn file_backends_satisfy_trait_contracts() {
        let dir = tmp_dir();
        let once = FileOnceStore::open(dir.join("o")).unwrap();
        once.ensure_available("c", true).unwrap();
        once.commit("c", true).unwrap();
        assert!(once.is_consumed("c"));
        let idem = FileIdemStore::open(dir.join("i")).unwrap();
        let r = ResultMsg::ok(vec![]);
        assert!(matches!(
            idem.put_or_check("k", "d", &r).unwrap(),
            IdemOutcome::Recorded
        ));
        let lin = FileLineageStore::open(dir.join("l")).unwrap();
        lin.commit(
            "c",
            Some("a"),
            "x",
            vec![],
            ReverseClass::NonReversible,
            vec![],
        )
        .unwrap();
        assert_eq!(lin.for_activity("a").unwrap().len(), 1);
        lin.mark_revoked("c").unwrap();
        assert!(lin.is_revoked("c"));
        drop(lin);
        let lin2 = FileLineageStore::open(dir.join("l")).unwrap();
        assert!(lin2.is_revoked("c"));
        assert_eq!(lin2.for_cap("c").unwrap().len(), 1);
        assert!(lin2.ensure_not_revoked("c").is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupt_json_fails_closed() {
        let dir = tmp_dir();
        std::fs::write(dir.join("once.json"), "{not-json").unwrap();
        assert!(FileOnceStore::open(&dir).is_err());
        std::fs::write(dir.join("idem.json"), "{not-json").unwrap();
        assert!(FileIdemStore::open(&dir).is_err());
        std::fs::write(dir.join("lineage.json"), "{not-json").unwrap();
        assert!(FileLineageStore::open(&dir).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn empty_file_is_default_not_down() {
        let dir = tmp_dir();
        std::fs::write(dir.join("once.json"), "   ").unwrap();
        let s = FileOnceStore::open(&dir).unwrap();
        assert!(!s.is_consumed("x"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn file_once_non_once_is_noop() {
        let dir = tmp_dir();
        let s = FileOnceStore::open(&dir).unwrap();
        s.ensure_available("n", false).unwrap();
        s.commit("n", false).unwrap();
        assert!(!s.is_consumed("n"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn annotate_unknown_entry_errors() {
        let dir = tmp_dir();
        let s = FileLineageStore::open(&dir).unwrap();
        assert!(s.annotate_landed("nope", vec![]).is_err());
        assert!(s
            .annotate_landed_latest_for_activity("missing", vec![])
            .is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
