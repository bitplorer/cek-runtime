//! Property-style tests for contract types, Baseline catalog, and serde.
#![cfg(test)]

use crate::{
    baseline, is_baseline, kv_set_inverse, Cap, FailClosed, Intent, Manifest, Op, Profile, Receipt,
    ResultKind, ResultMsg, ReverseClass, UnknownOpPolicy, LAW_GENERATION, PROFILE_BASELINE,
    PROFILE_PRODUCTION_V1,
};
use serde_json::json;
use std::collections::BTreeMap;

#[test]
fn prop_result_effect_free() {
    assert!(ResultMsg::authority_refusal("x").is_effect_free());
    assert!(ResultMsg::dispatch_error("x").is_effect_free());
    assert!(ResultMsg::ok(vec![]).is_effect_free());
    assert!(!ResultMsg::ok(vec![baseline::kv_set("k", json!(1))]).is_effect_free());
}

#[test]
fn prop_serde_roundtrip_core_types() {
    let cap = Cap {
        id: "c".into(),
        action: "kv.write".into(),
        sealed_args_bind: Some("cek1:abc".into()),
        not_after: Some(9),
        once: true,
        subject: Some("s".into()),
        scopes: vec!["narrow".into()],
        sig: None,
    };
    let mut args = BTreeMap::new();
    args.insert("key".into(), json!("k"));
    let intent = Intent {
        action: "kv.write".into(),
        args,
        cap: cap.clone(),
        trace: Some("t".into()),
        idempotency_key: Some("i".into()),
        activity_id: Some("a".into()),
    };
    let op = baseline::kv_set("k", json!(1));
    let result = ResultMsg {
        kind: ResultKind::Ok,
        ops: vec![op.clone()],
        error: None,
        digest: Some("cek1:x".into()),
    };
    for (name, v) in [
        ("cap", serde_json::to_value(&cap).unwrap()),
        ("intent", serde_json::to_value(&intent).unwrap()),
        ("op", serde_json::to_value(&op).unwrap()),
        ("result", serde_json::to_value(&result).unwrap()),
    ] {
        let back = serde_json::to_string(&v).unwrap();
        assert!(back.contains('{'), "{name}");
    }
    let cap2: Cap = serde_json::from_value(serde_json::to_value(&cap).unwrap()).unwrap();
    assert_eq!(cap, cap2);
    let intent2: Intent = serde_json::from_value(serde_json::to_value(&intent).unwrap()).unwrap();
    assert_eq!(intent, intent2);
}

#[test]
fn prop_unknown_json_fields_are_ignored() {
    let v = json!({
        "id": "c",
        "action": "kv.write",
        "once": false,
        "unknown_meta": { "x": 1 },
        "also_unknown": true
    });
    let cap: Cap = serde_json::from_value(v).unwrap();
    assert_eq!(cap.id, "c");
    assert_eq!(cap.action, "kv.write");
}

#[test]
fn prop_baseline_catalog() {
    assert!(is_baseline("kv", "set"));
    assert!(is_baseline("kv", "delete"));
    assert!(is_baseline("log", "append"));
    assert!(!is_baseline("ui", "morph"));
    assert!(!is_baseline("kv", "write"));
    assert_eq!(baseline::BASELINE_OPS.len(), 3);
    let inv_none = kv_set_inverse("k", None);
    assert_eq!(inv_none.fq(), "kv.delete");
    let inv_some = kv_set_inverse("k", Some(json!(1)));
    assert_eq!(inv_some.fq(), "kv.set");
}

#[test]
fn prop_op_fq_and_kind_rename() {
    let op = Op {
        ns: "kv".into(),
        name: "set".into(),
        payload: json!({}),
    };
    assert_eq!(op.fq(), "kv.set");
    let k = serde_json::to_string(&ResultKind::AuthorityRefusal).unwrap();
    assert_eq!(k, "\"authority_refusal\"");
    let rc = serde_json::to_string(&ReverseClass::NonReversible).unwrap();
    assert_eq!(rc, "\"non_reversible\"");
}

#[test]
fn constants_and_defaults() {
    assert_eq!(LAW_GENERATION, "cek-law-1");
    assert_eq!(PROFILE_BASELINE, "baseline");
    assert_eq!(PROFILE_PRODUCTION_V1, "production-v1");
    let fc = FailClosed::default();
    assert!(fc.once_store_down);
    assert!(fc.idem_store_down);
    assert!(fc.sealed_args);
    assert!(fc.scopes);
    assert!(!fc.cap_signatures);
    assert!(fc.idem_store_down);
    assert!(fc.sealed_args);
    assert!(fc.scopes);
    let p = UnknownOpPolicy::default();
    assert!(matches!(p, UnknownOpPolicy::Skip));
    let _ = Manifest {
        law_generation: LAW_GENERATION.into(),
        profiles: vec![PROFILE_BASELINE.into()],
        fail_closed: fc,
    };
    let _ = Profile {
        name: "baseline".into(),
        apply_set: vec!["kv.set".into()],
        unknown_op_policy: UnknownOpPolicy::FailBatch,
    };
    let _ = Receipt {
        landed: vec![],
        failed: vec![],
    };
}
