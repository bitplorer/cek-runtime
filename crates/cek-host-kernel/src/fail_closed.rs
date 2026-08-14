//! Fail-closed store behaviour: a down backend must never skip once / idem / lineage.
#![cfg(test)]

use crate::{
    Host, HostError, HostResult, IdemBackend, IdemOutcome, LineageBackend, LineageStore,
    OnceBackend, OnceStore,
};
use cek_contract::{baseline, Intent, LineageEntry, Op, ResultKind, ResultMsg, ReverseClass};
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::Arc;

struct DownOnce;

impl OnceBackend for DownOnce {
    fn ensure_available(&self, _cap_id: &str, once: bool) -> HostResult<()> {
        if once {
            Err(HostError::OnceStoreDown)
        } else {
            Ok(())
        }
    }
    fn commit(&self, _cap_id: &str, once: bool) -> HostResult<()> {
        if once {
            Err(HostError::OnceStoreDown)
        } else {
            Ok(())
        }
    }
    fn is_consumed(&self, _cap_id: &str) -> bool {
        false
    }
}

struct DownIdem;

impl IdemBackend for DownIdem {
    fn get(&self, _key: &str) -> HostResult<Option<ResultMsg>> {
        Err(HostError::IdemStoreDown)
    }
    fn put_or_check(
        &self,
        _key: &str,
        _digest: &str,
        _result: &ResultMsg,
    ) -> HostResult<IdemOutcome> {
        Err(HostError::IdemStoreDown)
    }
}

struct DownLineage;

impl LineageBackend for DownLineage {
    fn mark_ended(&self, _activity_id: &str) -> HostResult<()> {
        Err(HostError::Lineage("down".into()))
    }
    fn is_ended(&self, _activity_id: &str) -> bool {
        false
    }
    fn commit(
        &self,
        _cap_id: &str,
        _activity_id: Option<&str>,
        _action: &str,
        _authorized_ops: Vec<Op>,
        _reverse_class: ReverseClass,
        _inverse_ops: Vec<Op>,
    ) -> HostResult<LineageEntry> {
        Err(HostError::Lineage("down".into()))
    }
    fn annotate_landed(&self, _entry_id: &str, _landed: Vec<Op>) -> HostResult<()> {
        Err(HostError::Lineage("down".into()))
    }
    fn annotate_landed_latest_for_activity(
        &self,
        _activity_id: &str,
        _landed: Vec<Op>,
    ) -> HostResult<()> {
        Err(HostError::Lineage("down".into()))
    }
    fn for_activity(&self, _activity_id: &str) -> HostResult<Vec<LineageEntry>> {
        Err(HostError::Lineage("down".into()))
    }
}

fn write_intent(host: &Host, id: &str, once: bool, idem: Option<&str>) -> Intent {
    let cap = host.mint(id, "kv.write", once, None);
    let mut args = BTreeMap::new();
    args.insert("key".into(), json!("k"));
    args.insert("value".into(), json!(1));
    Intent {
        action: "kv.write".into(),
        args,
        cap,
        trace: None,
        idempotency_key: idem.map(|s| s.to_string()),
        activity_id: Some("act-down".into()),
    }
}

#[test]
fn once_store_down_refuses_once_cap() {
    let host = Host::with_stores(
        Arc::new(DownOnce),
        Arc::new(crate::IdemStore::new()),
        Arc::new(LineageStore::new()),
        1000,
    );
    let r = host.submit(write_intent(&host, "c-down", true, None));
    assert!(matches!(r.kind, ResultKind::AuthorityRefusal));
    assert!(r.ops.is_empty());
    assert!(r.error.as_deref().unwrap().contains("once store"));
}

#[test]
fn once_store_down_does_not_block_non_once() {
    let host = Host::with_stores(
        Arc::new(DownOnce),
        Arc::new(crate::IdemStore::new()),
        Arc::new(LineageStore::new()),
        1000,
    );
    let r = host.submit(write_intent(&host, "c-ok", false, None));
    assert!(matches!(r.kind, ResultKind::Ok));
}

#[test]
fn idem_store_down_refuses_when_key_present() {
    let host = Host::with_stores(
        Arc::new(OnceStore::new()),
        Arc::new(DownIdem),
        Arc::new(LineageStore::new()),
        1000,
    );
    let r = host.submit(write_intent(&host, "c-id", false, Some("ik")));
    assert!(matches!(r.kind, ResultKind::AuthorityRefusal));
    assert!(r.ops.is_empty());
    assert!(r.error.as_deref().unwrap().contains("idempotency"));
}

#[test]
fn lineage_store_down_is_dispatch_error_not_ok_with_ops_committed() {
    let host = Host::with_stores(
        Arc::new(OnceStore::new()),
        Arc::new(crate::IdemStore::new()),
        Arc::new(DownLineage),
        1000,
    );
    let r = host.submit(write_intent(&host, "c-lin", false, None));
    // Project succeeded; lineage commit failed → dispatch_error, zero new world
    // (Host does not apply; Peer never sees an ok Result).
    assert!(matches!(r.kind, ResultKind::DispatchError));
    assert!(r.ops.is_empty());
}

#[test]
fn report_receipt_and_end_fail_closed_when_lineage_down() {
    let host = Host::with_stores(
        Arc::new(OnceStore::new()),
        Arc::new(crate::IdemStore::new()),
        Arc::new(DownLineage),
        1000,
    );
    let rec = cek_contract::Receipt {
        landed: vec![baseline::kv_set("k", json!(1))],
        failed: vec![],
    };
    assert!(host.report_receipt("act", &rec).is_err());
    assert!(host.end_activity("act").is_err());
}

#[test]
fn end_empty_activity_id_errors() {
    let host = Host::with_clock(1000);
    assert!(host.end_activity("").is_err());
}

#[test]
fn concurrent_once_only_one_ok() {
    use std::sync::Arc as StdArc;
    let host = StdArc::new(Host::with_clock(1000));
    let cap = host.mint("once-race", "kv.write", true, None);
    let mut joins = Vec::new();
    for i in 0..8 {
        let h = StdArc::clone(&host);
        let c = cap.clone();
        joins.push(std::thread::spawn(move || {
            let mut args = BTreeMap::new();
            args.insert("key".into(), json!("k"));
            args.insert("value".into(), json!(i));
            h.submit(Intent {
                action: "kv.write".into(),
                args,
                cap: c,
                trace: None,
                idempotency_key: None,
                activity_id: None,
            })
        }));
    }
    let results: Vec<_> = joins.into_iter().map(|j| j.join().unwrap()).collect();
    let oks = results
        .iter()
        .filter(|r| matches!(r.kind, ResultKind::Ok))
        .count();
    let refuses = results
        .iter()
        .filter(|r| matches!(r.kind, ResultKind::AuthorityRefusal))
        .count();
    assert_eq!(oks, 1, "exactly one once-Cap must land");
    assert_eq!(refuses, 7);
    for r in &results {
        if matches!(r.kind, ResultKind::AuthorityRefusal) {
            assert!(r.ops.is_empty());
        }
        assert!(r.digest.is_some());
    }
}
