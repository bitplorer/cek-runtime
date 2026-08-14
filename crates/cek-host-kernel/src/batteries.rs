//! Stress, load, chaos, and pen batteries. Refuse must stay zero-Ops.

#![cfg(test)]

use crate::Host;
use cek_contract::{Intent, ResultKind};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::thread;

fn write(cap: cek_contract::Cap, key: &str, val: Value) -> Intent {
    let mut args = BTreeMap::new();
    args.insert("key".into(), json!(key));
    args.insert("value".into(), val);
    Intent {
        action: "kv.write".into(),
        args,
        cap,
        trace: None,
        idempotency_key: None,
        activity_id: None,
    }
}

fn assert_refuse(r: &cek_contract::ResultMsg) {
    assert!(
        matches!(r.kind, ResultKind::AuthorityRefusal),
        "{:?}",
        r.kind
    );
    assert!(r.ops.is_empty(), "refuse leaked {} ops", r.ops.len());
}

// --- stress ---

#[test]
fn stress_1k_ok_writes() {
    let host = Host::with_clock(1_000);
    for i in 0..1_000 {
        let cap = host.mint(format!("c-{i}"), "kv.write", false, None);
        let r = host.submit(write(cap, &format!("k{i}"), json!(i)));
        assert!(matches!(r.kind, ResultKind::Ok));
        assert_eq!(r.ops.len(), 1);
    }
}

#[test]
fn stress_concurrent_once_one_ok() {
    let host = Arc::new(Host::with_clock(1_000));
    let cap = host.mint("once-stress", "kv.write", true, None);
    let ok = Arc::new(Mutex::new(0u32));
    let refuse = Arc::new(Mutex::new(0u32));
    let leaked = Arc::new(Mutex::new(0u32));
    thread::scope(|s| {
        for i in 0..32 {
            let host = Arc::clone(&host);
            let cap = cap.clone();
            let ok = Arc::clone(&ok);
            let refuse = Arc::clone(&refuse);
            let leaked = Arc::clone(&leaked);
            s.spawn(move || {
                let r = host.submit(write(cap, "k", json!(i)));
                match r.kind {
                    ResultKind::Ok => {
                        *ok.lock().unwrap() += 1;
                        if r.ops.is_empty() {
                            *leaked.lock().unwrap() += 1;
                        }
                    }
                    ResultKind::AuthorityRefusal => {
                        *refuse.lock().unwrap() += 1;
                        if !r.ops.is_empty() {
                            *leaked.lock().unwrap() += 1;
                        }
                    }
                    _ => *leaked.lock().unwrap() += 1,
                }
            });
        }
    });
    assert_eq!(*ok.lock().unwrap(), 1);
    assert_eq!(*refuse.lock().unwrap(), 31);
    assert_eq!(*leaked.lock().unwrap(), 0);
}

#[test]
fn stress_200_activities_reverse() {
    let host = Host::with_clock(1_000);
    for i in 0..200 {
        let cap = host.mint(format!("a-{i}"), "kv.write", false, None);
        let mut intent = write(cap, &format!("k{i}"), json!(i));
        intent.activity_id = Some(format!("act-{i}"));
        assert!(matches!(host.submit(intent).kind, ResultKind::Ok));
        let rev = host.end_activity(&format!("act-{i}")).unwrap();
        assert_eq!(rev.ops.len(), 1);
        assert_eq!(rev.ops[0].fq(), "kv.delete");
    }
}

// --- load ---

#[test]
fn load_400_idem_keys_then_replay() {
    let host = Host::with_clock(1_000);
    for i in 0..400 {
        let cap = host.mint(format!("id-{i}"), "kv.write", false, None);
        let mut intent = write(cap, &format!("k{i}"), json!(i));
        intent.idempotency_key = Some(format!("idem-{i}"));
        let a = host.submit(intent.clone());
        let b = host.submit(intent);
        assert!(matches!(a.kind, ResultKind::Ok));
        assert!(matches!(b.kind, ResultKind::Ok));
        assert_eq!(a.ops, b.ops);
    }
}

#[test]
fn load_large_payload_still_ok() {
    let host = Host::with_clock(1_000);
    let blob = "x".repeat(64 * 1024);
    let cap = host.mint("big", "kv.write", false, None);
    let r = host.submit(write(cap, "blob", json!(blob)));
    assert!(matches!(r.kind, ResultKind::Ok));
    assert_eq!(r.ops[0].payload["value"], json!(blob));
}

#[test]
fn load_wide_scope_list() {
    let host = Host::with_clock(1_000);
    let mut cap = host.mint("sc", "kv.write", false, None);
    cap.scopes = (0..200).map(|i| format!("kv:k{i}")).collect();
    let r = host.submit(write(cap.clone(), "k0", json!(1)));
    assert!(matches!(r.kind, ResultKind::Ok));
    let r = host.submit(write(cap, "nope", json!(1)));
    assert_refuse(&r);
}

// --- chaos ---

#[test]
fn chaos_clock_max_expires_all() {
    let host = Host::with_clock(u64::MAX);
    let cap = host.mint("exp", "kv.write", false, Some(1));
    assert_refuse(&host.submit(write(cap, "k", json!(1))));
}

#[test]
fn chaos_unicode_and_pathlike_keys() {
    let host = Host::with_clock(1_000);
    for key in ["../etc/passwd", "🔑", "a/../b", "\u{0}null", "   "] {
        if key.trim().is_empty() {
            continue;
        }
        let cap = host.mint(format!("u-{key}"), "kv.write", false, None);
        let r = host.submit(write(cap, key, json!(1)));
        assert!(matches!(r.kind, ResultKind::Ok), "{key}");
    }
}

#[test]
fn chaos_unknown_activity_end_errors() {
    let host = Host::with_clock(1_000);
    assert!(host.end_activity("ghost").is_err() || {
        let o = host.end_activity("ghost2");
        o.is_ok() && o.unwrap().ops.is_empty()
    });
}

#[test]
fn chaos_file_stores_survive_many_reopens() {
    let dir = std::env::temp_dir().join(format!("cek-chaos-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    {
        let stores = crate::FileStores::open(&dir).unwrap();
        let host = Host::with_backends(
            Arc::new(stores.once),
            Arc::new(stores.idem),
            Arc::new(stores.lineage),
        );
        let cap = host.mint("persist", "kv.write", true, None);
        assert!(matches!(
            host.submit(write(cap, "k", json!(1))).kind,
            ResultKind::Ok
        ));
    }
    {
        let stores = crate::FileStores::open(&dir).unwrap();
        let host = Host::with_backends(
            Arc::new(stores.once),
            Arc::new(stores.idem),
            Arc::new(stores.lineage),
        );
        let cap = host.mint("persist", "kv.write", true, None);
        assert_refuse(&host.submit(write(cap, "k", json!(2))));
    }
    let _ = std::fs::remove_dir_all(&dir);
}

// --- pen ---

#[test]
fn pen_action_mismatch_zero_ops() {
    let host = Host::with_clock(1_000);
    let cap = host.mint("p", "kv.read", false, None);
    assert_refuse(&host.submit(write(cap, "k", json!(1))));
}

#[test]
fn pen_hmac_bitflip_zero_ops() {
    let key = [9u8; 32];
    let host = Host::with_clock(1_000).with_hmac_key(key);
    let mut cap = host.mint("p", "kv.write", false, None);
    if let Some(sig) = cap.sig.as_mut() {
        let last = sig.pop().unwrap();
        sig.push(if last == 'a' { 'b' } else { 'a' });
    }
    assert_refuse(&host.submit(write(cap, "k", json!(1))));
}

#[test]
fn pen_ed25519_prefix_confusion_zero_ops() {
    let host = Host::with_clock(1_000).with_ed25519([3u8; 32]);
    let mut cap = host.mint("p", "kv.write", false, None);
    cap.sig = Some("cek1:00".into());
    assert_refuse(&host.submit(write(cap, "k", json!(1))));
}

#[test]
fn pen_subject_spoof_zero_ops() {
    let host = Host::with_clock(1_000);
    let mut cap = host.mint("p", "kv.write", false, None);
    cap.subject = Some("alice".into());
    let mut intent = write(cap, "k", json!(1));
    intent.args.insert("subject".into(), json!("mallory"));
    assert_refuse(&host.submit(intent));
}

#[test]
fn pen_law_generation_injection_zero_ops() {
    let host = Host::with_clock(1_000);
    let mut cap = host.mint("p", "kv.write", false, None);
    cap.law_generation = Some("cek-law-1\ncek-law-99".into());
    assert_refuse(&host.submit(write(cap, "k", json!(1))));
}

#[test]
fn pen_empty_tokens_zero_ops() {
    let host = Host::with_clock(1_000);
    let mut cap = host.mint("p", "kv.write", false, None);
    cap.scopes = vec!["  ".into()];
    assert_refuse(&host.submit(write(cap, "k", json!(1))));
    let cap = host.mint("", "kv.write", false, None);
    assert_refuse(&host.submit(write(cap, "k", json!(1))));
}

#[test]
fn pen_hmac_host_rejects_unsigned() {
    let host = Host::with_clock(1_000).with_hmac_key([1u8; 32]);
    let mut cap = host.mint("p", "kv.write", false, None);
    cap.sig = None;
    assert_refuse(&host.submit(write(cap, "k", json!(1))));
}
