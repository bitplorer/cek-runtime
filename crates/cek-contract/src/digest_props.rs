//! Property-style digest tests (deterministic case table).
#![cfg(test)]

use crate::{ops_digest, result_digest, sealed_args_digest, Op};
use serde_json::json;
use std::collections::BTreeMap;

#[test]
fn prop_ops_digest_deterministic() {
    for ns in ["kv", "log", "ui"] {
        for name in ["set", "append", "morph"] {
            for n in [0u32, 1, 99] {
                let op = Op {
                    ns: ns.into(),
                    name: name.into(),
                    payload: json!({ "n": n }),
                };
                let a = ops_digest(&[op.clone()]);
                let b = ops_digest(&[op]);
                assert_eq!(a, b);
                assert!(a.starts_with("cek1:"));
            }
        }
    }
}

#[test]
fn prop_sealed_order_irrelevant() {
    let pairs = [("aa", "bb"), ("key", "other"), ("z", "a")];
    for (k1, k2) in pairs {
        let mut a = BTreeMap::new();
        a.insert(k1.into(), json!(1));
        a.insert(k2.into(), json!(2));
        let mut b = BTreeMap::new();
        b.insert(k2.into(), json!(2));
        b.insert(k1.into(), json!(1));
        assert_eq!(sealed_args_digest(&a), sealed_args_digest(&b));
    }
}

#[test]
fn prop_refusal_digest_stable() {
    for msg in [
        "",
        "no",
        "action mismatch",
        "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
    ] {
        let d1 = result_digest("authority_refusal", &[], Some(&msg));
        let d2 = result_digest("authority_refusal", &[], Some(&msg));
        assert_eq!(d1, d2);
        assert!(d1.starts_with("cek1:"));
    }
}
