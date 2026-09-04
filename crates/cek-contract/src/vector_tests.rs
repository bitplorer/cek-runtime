//! Vector loader / checker unit tests.
#![cfg(test)]

use crate::{baseline, check_result, load_vector_dir, load_vector_file, ResultMsg, VectorCase};
use serde_json::json;
use std::collections::BTreeMap;

fn tmp_json(name: &str, body: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("cek-vec-{}-{}", std::process::id(), name));
    let _ = std::fs::create_dir_all(&dir);
    let p = dir.join(format!("{name}.json"));
    std::fs::write(&p, body).unwrap();
    p
}

#[test]
fn load_rejects_missing_id() {
    let p = tmp_json(
        "noid",
        r#"{"id":"","family":"x","description":"d","expect_kind":"ok"}"#,
    );
    assert!(load_vector_file(&p).is_err());
}

#[test]
fn load_rejects_bad_json() {
    let p = tmp_json("bad", "{not json");
    assert!(load_vector_file(&p).is_err());
}

#[test]
fn check_result_kind_mismatch() {
    let case = VectorCase {
        id: "c".into(),
        family: "f".into(),
        description: "d".into(),
        intent: None,
        prior_intent: None,
        prior_must_ok: false,
        prior_end_activity: None,
        now: None,
        expect_kind: "ok".into(),
        expect_ops_empty: false,
        expect_ops: None,
        peer_apply: false,
        peer_result: None,
        peer_unknown_policy: None,
        expect_peer_kv: None,
        report_receipt: false,
        end_activity: None,
        expect_reverse_ops: None,
        expect_used_landed: None,
        end_activity_again: false,
        expect_once_consumed: None,
        peer_profile: None,
        expect_peer_ui: None,
        expect_lowered_ops: None,
        hmac_key: None,
        sign_cap: false,
        ed25519_seed: None,
        accept_generations: None,
        revoke_cap: None,
        expect_revoke_reverse_ops: None,
        expect_revoke_non_reversible: false,
        revoke_again: false,
        mint_recovery: None,
        compensation_commit: None,
        expect_end_non_reversible: false,
    };
    let r = ResultMsg::authority_refusal("no");
    assert!(check_result(&case, &r).is_err());
}

#[test]
fn check_result_rejects_refusal_with_ops() {
    let case = VectorCase {
        id: "c".into(),
        family: "f".into(),
        description: "d".into(),
        intent: None,
        prior_intent: None,
        prior_must_ok: false,
        prior_end_activity: None,
        now: None,
        expect_kind: "authority_refusal".into(),
        expect_ops_empty: false,
        expect_ops: None,
        peer_apply: false,
        peer_result: None,
        peer_unknown_policy: None,
        expect_peer_kv: None,
        report_receipt: false,
        end_activity: None,
        expect_reverse_ops: None,
        expect_used_landed: None,
        end_activity_again: false,
        expect_once_consumed: None,
        peer_profile: None,
        expect_peer_ui: None,
        expect_lowered_ops: None,
        hmac_key: None,
        sign_cap: false,
        ed25519_seed: None,
        accept_generations: None,
        revoke_cap: None,
        expect_revoke_reverse_ops: None,
        expect_revoke_non_reversible: false,
        revoke_again: false,
        mint_recovery: None,
        compensation_commit: None,
        expect_end_non_reversible: false,
    };
    let mut r = ResultMsg::authority_refusal("no");
    r.ops = vec![baseline::kv_set("k", json!(1))];
    assert!(check_result(&case, &r).is_err());
}

#[test]
fn check_result_ops_empty_and_exact() {
    let mut case = VectorCase {
        id: "c".into(),
        family: "f".into(),
        description: "d".into(),
        intent: None,
        prior_intent: None,
        prior_must_ok: false,
        prior_end_activity: None,
        now: None,
        expect_kind: "ok".into(),
        expect_ops_empty: true,
        expect_ops: None,
        peer_apply: false,
        peer_result: None,
        peer_unknown_policy: None,
        expect_peer_kv: None,
        report_receipt: false,
        end_activity: None,
        expect_reverse_ops: None,
        expect_used_landed: None,
        end_activity_again: false,
        expect_once_consumed: None,
        peer_profile: None,
        expect_peer_ui: None,
        expect_lowered_ops: None,
        hmac_key: None,
        sign_cap: false,
        ed25519_seed: None,
        accept_generations: None,
        revoke_cap: None,
        expect_revoke_reverse_ops: None,
        expect_revoke_non_reversible: false,
        revoke_again: false,
        mint_recovery: None,
        compensation_commit: None,
        expect_end_non_reversible: false,
    };
    let with_ops = ResultMsg::ok(vec![baseline::kv_set("k", json!(1))]);
    assert!(check_result(&case, &with_ops).is_err());
    case.expect_ops_empty = false;
    case.expect_ops = Some(vec![baseline::kv_delete("k")]);
    assert!(check_result(&case, &with_ops).is_err());
    case.expect_ops = Some(with_ops.ops.clone());
    assert!(check_result(&case, &with_ops).is_ok());
}

#[test]
fn load_vector_dir_reads_workspace_fixtures() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("vectors");
    let cases = load_vector_dir(&dir).unwrap();
    assert!(
        cases.len() >= 25,
        "expected at least 25 CORE vectors, got {}",
        cases.len()
    );
    let mut ids = BTreeMap::new();
    for (_, c) in &cases {
        assert!(!c.id.is_empty());
        assert!(!c.family.is_empty());
        assert!(
            ids.insert(c.id.clone(), ()).is_none(),
            "duplicate vector id {}",
            c.id
        );
    }
}
