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

/// ∀ mismatch(action, Cap.action) → refusal ∧ ops=∅ ∧ digest present
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
            assert!(r.digest.as_ref().unwrap().starts_with("cek1:"));
        }
    }
}

/// ∀ valid kv.write → ops=[kv.set] with same key
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

/// ∀ once Cap → second submit refuses with zero Ops
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

/// ∀ identical projections → identical digests (Cap id is not in the digest)
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

/// ∀ expired Cap → refusal ∧ ops=∅
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

/// ∀ kv.delete → ops=[kv.delete] with same key
#[test]
fn prop_kv_delete_projects() {
    let host = Host::with_clock(1_000);
    for key in keys() {
        let cap = host.mint(format!("del-{key}"), "kv.delete", false, None);
        let mut args = BTreeMap::new();
        args.insert("key".into(), json!(key));
        let r = host.submit(Intent {
            action: "kv.delete".into(),
            args,
            cap,
            trace: None,
            idempotency_key: None,
            activity_id: None,
        });
        assert!(matches!(r.kind, ResultKind::Ok), "{key}");
        assert_eq!(r.ops.len(), 1);
        assert_eq!(r.ops[0].fq(), "kv.delete");
        assert_eq!(
            r.ops[0].payload.get("key").and_then(|v| v.as_str()),
            Some(key)
        );
    }
}

/// ∀ log.append → ops=[log.append] with same message
#[test]
fn prop_log_append_projects() {
    let host = Host::with_clock(1_000);
    for (i, msg) in ["", "hello", "unicode-Δ", "x".repeat(64).as_str()]
        .iter()
        .enumerate()
    {
        let cap = host.mint(format!("log-{i}"), "log.append", false, None);
        let mut args = BTreeMap::new();
        args.insert("message".into(), json!(msg));
        let r = host.submit(Intent {
            action: "log.append".into(),
            args,
            cap,
            trace: None,
            idempotency_key: None,
            activity_id: None,
        });
        assert!(matches!(r.kind, ResultKind::Ok), "{msg}");
        assert_eq!(r.ops[0].fq(), "log.append");
        assert_eq!(
            r.ops[0].payload.get("message").and_then(|v| v.as_str()),
            Some(*msg)
        );
    }
}

/// ∀ kv.set under an Activity → reverse is kv.delete of that key
#[test]
fn prop_reverse_is_inverse_delete() {
    let host = Host::with_clock(1_000);
    for key in keys() {
        let cap = host.mint(format!("rev-{key}"), "kv.write", false, None);
        let mut args = BTreeMap::new();
        args.insert("key".into(), json!(key));
        args.insert("value".into(), json!(1));
        let aid = format!("act-{key}");
        let r = host.submit(Intent {
            action: "kv.write".into(),
            args,
            cap,
            trace: None,
            idempotency_key: None,
            activity_id: Some(aid.clone()),
        });
        assert!(matches!(r.kind, ResultKind::Ok));
        let rev = host.end_activity(&aid).unwrap();
        assert_eq!(rev.ops.len(), 1);
        assert_eq!(rev.ops[0].fq(), "kv.delete");
        assert_eq!(
            rev.ops[0].payload.get("key").and_then(|v| v.as_str()),
            Some(key)
        );
        assert!(!rev.used_landed);
    }
}

/// ∀ same idempotency key + same body → same digest; different body → refuse
#[test]
fn prop_idempotency_replay_and_conflict() {
    let host = Host::with_clock(1_000);
    for key in keys() {
        let cap = host.mint(format!("id-{key}"), "kv.write", false, None);
        let mut args = BTreeMap::new();
        args.insert("key".into(), json!(key));
        args.insert("value".into(), json!(1));
        let ik = format!("idem-{key}");
        let i1 = Intent {
            action: "kv.write".into(),
            args: args.clone(),
            cap: cap.clone(),
            trace: None,
            idempotency_key: Some(ik.clone()),
            activity_id: None,
        };
        let r1 = host.submit(i1.clone());
        let r2 = host.submit(i1);
        assert!(matches!(r1.kind, ResultKind::Ok));
        assert_eq!(r1.digest, r2.digest);
        args.insert("value".into(), json!(99));
        let i3 = Intent {
            action: "kv.write".into(),
            args,
            cap,
            trace: None,
            idempotency_key: Some(ik),
            activity_id: None,
        };
        let r3 = host.submit(i3);
        assert!(matches!(r3.kind, ResultKind::AuthorityRefusal));
        assert!(r3.ops.is_empty());
    }
}

/// ∀ sealed tamper → refuse; ∀ sealed match → ok
#[test]
fn prop_sealed_args_bind() {
    let host = Host::with_clock(1_000);
    for key in keys() {
        let mut sealed = BTreeMap::new();
        sealed.insert("key".into(), json!(key));
        sealed.insert("value".into(), json!(1));
        let cap = host.mint_sealed(format!("s-{key}"), "kv.write", false, None, &sealed);
        let ok = host.submit(Intent {
            action: "kv.write".into(),
            args: sealed.clone(),
            cap: cap.clone(),
            trace: None,
            idempotency_key: None,
            activity_id: None,
        });
        assert!(matches!(ok.kind, ResultKind::Ok), "{key}");
        let mut tamper = sealed;
        tamper.insert("value".into(), json!(2));
        let bad = host.submit(Intent {
            action: "kv.write".into(),
            args: tamper,
            cap,
            trace: None,
            idempotency_key: None,
            activity_id: None,
        });
        assert!(matches!(bad.kind, ResultKind::AuthorityRefusal));
        assert!(bad.ops.is_empty());
    }
}

/// ∀ trace string → never upgrades a mismatch into ok
#[test]
fn prop_trace_never_grants_authority() {
    let host = Host::with_clock(1_000);
    for (i, tr) in ["", "t", "shared", "unicode-Ω"].iter().enumerate() {
        let cap = host.mint(format!("tr-{i}"), "kv.read", false, None);
        let mut args = BTreeMap::new();
        args.insert("key".into(), json!("k"));
        args.insert("value".into(), json!(1));
        let r = host.submit(Intent {
            action: "kv.write".into(),
            args,
            cap,
            trace: Some((*tr).into()),
            idempotency_key: None,
            activity_id: None,
        });
        assert!(matches!(r.kind, ResultKind::AuthorityRefusal));
        assert!(r.ops.is_empty());
    }
}

/// ∀ unknown action on a once-Cap → dispatch_error and Cap not burned
#[test]
fn prop_once_not_burned_on_dispatch_error() {
    let host = Host::with_clock(1_000);
    for key in keys() {
        let id = format!("once-miss-{key}");
        let cap = host.mint(&id, "no.such.action", true, None);
        let r = host.submit(Intent {
            action: "no.such.action".into(),
            args: BTreeMap::new(),
            cap,
            trace: None,
            idempotency_key: None,
            activity_id: None,
        });
        assert!(matches!(r.kind, ResultKind::DispatchError));
        assert!(r.ops.is_empty());
        assert!(!host.once_store().is_consumed(&id));
    }
}

/// ∀ once-Cap + same idempotency key → retry returns cached ok
#[test]
fn prop_once_idempotent_retry() {
    let host = Host::with_clock(1_000);
    for key in keys() {
        let cap = host.mint(format!("oi-{key}"), "kv.write", true, None);
        let mut args = BTreeMap::new();
        args.insert("key".into(), json!(key));
        args.insert("value".into(), json!(1));
        let i = Intent {
            action: "kv.write".into(),
            args,
            cap,
            trace: None,
            idempotency_key: Some(format!("retry-{key}")),
            activity_id: None,
        };
        let r1 = host.submit(i.clone());
        let r2 = host.submit(i);
        assert!(matches!(r1.kind, ResultKind::Ok));
        assert!(matches!(r2.kind, ResultKind::Ok));
        assert_eq!(r1.digest, r2.digest);
        assert_eq!(r1.ops, r2.ops);
    }
}

/// Refusal kinds are always effect-free, for every refusal family we generate.
#[test]
fn prop_every_refusal_is_effect_free() {
    let host = Host::with_clock(2_000);
    let families = [
        intent_write(&host, "ef-m", "kv.read", "kv.write", "k", 1),
        {
            let cap = host.mint("ef-exp", "kv.write", false, Some(1));
            intent_write(&host, "ef-exp-unused", "kv.write", "kv.write", "k", 1).pipe_cap(cap)
        },
    ];
    // explicit expired
    let cap = host.mint("ef-e", "kv.write", false, Some(10));
    let mut args = BTreeMap::new();
    args.insert("key".into(), json!("k"));
    args.insert("value".into(), json!(1));
    let expired = Intent {
        action: "kv.write".into(),
        args,
        cap,
        trace: None,
        idempotency_key: None,
        activity_id: None,
    };
    for intent in families.into_iter().chain(std::iter::once(expired)) {
        let r = host.submit(intent);
        if matches!(r.kind, ResultKind::AuthorityRefusal) {
            assert!(r.ops.is_empty());
            assert!(r.is_effect_free());
        }
    }
}

trait PipeCap {
    fn pipe_cap(self, cap: cek_contract::Cap) -> Intent;
}

impl PipeCap for Intent {
    fn pipe_cap(mut self, cap: cek_contract::Cap) -> Intent {
        self.cap = cap;
        self
    }
}

/// ∀ ui.morph with snapshot → reverse is ui.dom.restore of that snapshot
#[test]
fn prop_ui_snapshot_reverse() {
    let host = Host::with_clock(1_000);
    for target in keys() {
        let cap = host.mint(format!("ui-{target}"), "ui.morph", false, None);
        let mut args = BTreeMap::new();
        args.insert("target".into(), json!(target));
        args.insert("patch".into(), json!({"v": 2}));
        args.insert("snapshot".into(), json!({"v": 1}));
        let aid = format!("act-ui-{target}");
        let r = host.submit(Intent {
            action: "ui.morph".into(),
            args,
            cap,
            trace: None,
            idempotency_key: None,
            activity_id: Some(aid.clone()),
        });
        assert!(matches!(r.kind, ResultKind::Ok), "{target}");
        assert_eq!(r.ops[0].fq(), "ui.dom.morph");
        let rev = host.end_activity(&aid).unwrap();
        assert_eq!(rev.ops.len(), 1);
        assert_eq!(rev.ops[0].fq(), "ui.dom.restore");
        assert_eq!(
            rev.ops[0].payload.get("target").and_then(|v| v.as_str()),
            Some(target)
        );
    }
}

/// ∀ scope that does not cover the key → refuse ∧ ops=∅
#[test]
fn prop_scope_deny_never_effects() {
    let host = Host::with_clock(1_000);
    for key in keys() {
        let mut cap = host.mint(format!("sc-{key}"), "kv.write", false, None);
        cap.scopes = vec!["kv:__none__".into()];
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

/// Attenuation never widens a non-empty parent.
#[test]
fn prop_attenuate_no_widen() {
    let host = Host::with_clock(1_000);
    for key in keys() {
        let mut parent = host.mint(format!("p-{key}"), "kv.write", false, None);
        parent.scopes = vec![format!("kv:{key}")];
        assert!(host
            .attenuate(&parent, format!("ok-{key}"), vec![format!("kv:{key}")])
            .is_ok());
        assert!(host
            .attenuate(&parent, format!("bad-{key}"), vec!["kv:__other__".into()])
            .is_err());
    }
}

/// ∀ kv.delete with prior → reverse is kv.set of that prior
#[test]
fn prop_kv_delete_prior_reverse() {
    let host = Host::with_clock(1_000);
    for key in keys() {
        let cap = host.mint(format!("delp-{key}"), "kv.delete", false, None);
        let mut args = BTreeMap::new();
        args.insert("key".into(), json!(key));
        args.insert("prior".into(), json!(format!("old-{key}")));
        let aid = format!("act-del-{key}");
        let r = host.submit(Intent {
            action: "kv.delete".into(),
            args,
            cap,
            trace: None,
            idempotency_key: None,
            activity_id: Some(aid.clone()),
        });
        assert!(matches!(r.kind, ResultKind::Ok), "{key}");
        let rev = host.end_activity(&aid).unwrap();
        assert_eq!(rev.ops.len(), 1);
        assert_eq!(rev.ops[0].fq(), "kv.set");
        assert_eq!(
            rev.ops[0].payload.get("key").and_then(|v| v.as_str()),
            Some(key)
        );
    }
}

/// ∀ kv.delete without prior → reverse is empty (honest)
#[test]
fn prop_kv_delete_no_prior_non_reversible() {
    let host = Host::with_clock(1_000);
    for key in keys() {
        let cap = host.mint(format!("deln-{key}"), "kv.delete", false, None);
        let mut args = BTreeMap::new();
        args.insert("key".into(), json!(key));
        let aid = format!("act-deln-{key}");
        let r = host.submit(Intent {
            action: "kv.delete".into(),
            args,
            cap,
            trace: None,
            idempotency_key: None,
            activity_id: Some(aid.clone()),
        });
        assert!(matches!(r.kind, ResultKind::Ok), "{key}");
        let rev = host.end_activity(&aid).unwrap();
        assert!(rev.ops.is_empty());
        assert!(!rev.non_reversible.is_empty());
    }
}

/// ∀ Host with HMAC key: unsigned / tampered Cap → refuse ∧ ops=∅; minted → ok
#[test]
fn prop_cap_hmac_never_effects_on_bad_sig() {
    let key = [0x11u8; 32];
    let host = Host::with_clock(1_000).with_hmac_key(key);
    for (i, k) in keys().iter().enumerate() {
        let good = host.mint(format!("sig-{i}-{k}"), "kv.write", false, None);
        let mut args = BTreeMap::new();
        args.insert("key".into(), json!(k));
        args.insert("value".into(), json!(1));
        let r = host.submit(Intent {
            action: "kv.write".into(),
            args: args.clone(),
            cap: good.clone(),
            trace: None,
            idempotency_key: None,
            activity_id: None,
        });
        assert!(matches!(r.kind, ResultKind::Ok), "{k}");

        let mut unsigned = good.clone();
        unsigned.sig = None;
        let r = host.submit(Intent {
            action: "kv.write".into(),
            args: args.clone(),
            cap: unsigned,
            trace: None,
            idempotency_key: None,
            activity_id: None,
        });
        assert!(matches!(r.kind, ResultKind::AuthorityRefusal), "{k}");
        assert!(r.ops.is_empty());

        let mut tamper = good;
        tamper.sig = Some("cek1:ff".into());
        let r = host.submit(Intent {
            action: "kv.write".into(),
            args,
            cap: tamper,
            trace: None,
            idempotency_key: None,
            activity_id: None,
        });
        assert!(matches!(r.kind, ResultKind::AuthorityRefusal), "{k}");
        assert!(r.ops.is_empty());
    }
}
