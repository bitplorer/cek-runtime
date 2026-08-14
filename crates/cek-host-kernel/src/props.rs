//! Property-style tests for Host invariants (no external proptest).
//!
//! Uses a fixed set of generated cases so CI is deterministic and
//! toolchain-independent.
#![cfg(test)]

use crate::Host;
use cek_contract::{Intent, ResultKind};
use serde_json::json;
use std::collections::BTreeMap;

fn keys() -> Vec<&'static str> {
    vec!["a", "b", "key1", "x_y", "Z9", "long_key_name"]
}

fn intent_write(
    host: &Host,
    cap_id: &str,
    action_cap: &str,
    action_intent: &str,
    key: &str,
    val: i64,
) -> Intent {
    let cap = host.mint(cap_id, action_cap, false, None);
    let mut args = BTreeMap::new();
    args.insert("key".into(), json!(key));
    args.insert("value".into(), json!(val));
    Intent {
        action: action_intent.into(),
        args,
        cap,
        trace: None,
        idempotency_key: None,
        activity_id: None,
    }
}

#[test]
fn prop_action_mismatch_never_effects() {
    let host = Host::with_clock(1_000);
    for (i, key) in keys().iter().enumerate() {
        for val in [-10, 0, 42] {
            let intent = intent_write(
                &host,
                &format!("m-{i}-{key}-{val}"),
                "kv.read",
                "kv.write",
                key,
                val,
            );
            let r = host.submit(intent);
            assert!(matches!(r.kind, ResultKind::AuthorityRefusal));
            assert!(r.ops.is_empty());
            assert!(r.digest.is_some());
        }
    }
}

#[test]
fn prop_kv_write_projects_set() {
    let host = Host::with_clock(1_000);
    for (i, key) in keys().iter().enumerate() {
        for val in [-3, 0, 7, 100] {
            let intent = intent_write(
                &host,
                &format!("w-{i}-{key}-{val}"),
                "kv.write",
                "kv.write",
                key,
                val,
            );
            let r = host.submit(intent);
            assert!(matches!(r.kind, ResultKind::Ok), "{key} {val}");
            assert_eq!(r.ops.len(), 1);
            assert_eq!(r.ops[0].fq(), "kv.set");
            assert_eq!(
                r.ops[0].payload.get("key").and_then(|v| v.as_str()),
                Some(*key)
            );
        }
    }
}

#[test]
fn prop_once_second_refuses() {
    let host = Host::with_clock(1_000);
    for key in keys() {
        let cap = host.mint(format!("once-{key}"), "kv.write", true, None);
        let mut args = BTreeMap::new();
        args.insert("key".into(), json!(key));
        args.insert("value".into(), json!(1));
        let i1 = Intent {
            action: "kv.write".into(),
            args: args.clone(),
            cap: cap.clone(),
            trace: None,
            idempotency_key: None,
            activity_id: None,
        };
        let r1 = host.submit(i1);
        assert!(matches!(r1.kind, ResultKind::Ok));
        let i2 = Intent {
            action: "kv.write".into(),
            args,
            cap,
            trace: None,
            idempotency_key: None,
            activity_id: None,
        };
        let r2 = host.submit(i2);
        assert!(matches!(r2.kind, ResultKind::AuthorityRefusal));
        assert!(r2.ops.is_empty());
    }
}

#[test]
fn prop_digest_stable_across_caps() {
    let host = Host::with_clock(1_000);
    for key in keys() {
        let build = |suffix: &str| {
            intent_write(
                &host,
                &format!("d-{key}-{suffix}"),
                "kv.write",
                "kv.write",
                key,
                1,
            )
        };
        let r1 = host.submit(build("a"));
        let r2 = host.submit(build("b"));
        assert!(matches!(r1.kind, ResultKind::Ok));
        assert!(matches!(r2.kind, ResultKind::Ok));
        assert_eq!(r1.digest, r2.digest);
        assert_eq!(r1.ops, r2.ops);
    }
}

#[test]
fn prop_expired_never_effects() {
    let host = Host::with_clock(5_000);
    for key in keys() {
        let cap = host.mint(format!("exp-{key}"), "kv.write", false, Some(1000));
        let mut args = BTreeMap::new();
        args.insert("key".into(), json!(key));
        args.insert("value".into(), json!(1));
        let r = host.submit(Intent {
            action: "kv.write".into(),
            args,
            cap,
            trace: None,
            idempotency_key: None,
            activity_id: None,
        });
        assert!(matches!(r.kind, ResultKind::AuthorityRefusal));
        assert!(r.ops.is_empty());
    }
}
