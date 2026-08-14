//! cek CLI — demo S1 and vector runner.

use cek_contract::{
    check_result, load_vector_dir, sealed_args_digest, Intent, ResultKind, ResultMsg,
    UnknownOpPolicy, VectorCase,
};
use cek_host_kernel::Host;
use cek_peer_kernel::Peer;
use serde_json::json;
use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;

fn main() {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("demo") => run_demo(),
        Some("vectors") => {
            let dir = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("crates/cek-contract/vectors"));
            run_vectors(&dir);
        }
        _ => {
            eprintln!("Usage:\n  cek demo\n  cek vectors [dir]");
            std::process::exit(2);
        }
    }
}

fn run_demo() {
    println!("=== CEK mature demo (Host + Peer) ===\n");
    let host = Host::with_clock(1_000);
    let peer = Peer::baseline();

    // 1) Refuse path
    let bad_cap = host.mint("cap-bad", "kv.read", false, None);
    let mut args = BTreeMap::new();
    args.insert("key".into(), json!("x"));
    args.insert("value".into(), json!(1));
    let bad = Intent {
        action: "kv.write".into(),
        args: args.clone(),
        cap: bad_cap,
        trace: None,
        idempotency_key: None,
        activity_id: None,
    };
    let r = host.submit(bad);
    println!("1) action mismatch → {:?} digest={:?}", r.kind, r.digest);
    assert!(matches!(r.kind, ResultKind::AuthorityRefusal));
    assert!(r.ops.is_empty());
    let _ = peer.apply(&r);
    assert!(peer.kv_get("x").is_none());
    println!("   world unchanged: ok\n");

    // 2) Sealed args
    let mut sealed = BTreeMap::new();
    sealed.insert("key".into(), json!("greeting"));
    sealed.insert("value".into(), json!("hello"));
    let bind = sealed_args_digest(&sealed);
    println!("2) sealed bind = {bind}");
    let cap_seal = host.mint_sealed("cap-seal", "kv.write", false, None, &sealed);
    let mut tamper = sealed.clone();
    tamper.insert("value".into(), json!("evil"));
    let r = host.submit(Intent {
        action: "kv.write".into(),
        args: tamper,
        cap: cap_seal.clone(),
        trace: None,
        idempotency_key: None,
        activity_id: None,
    });
    println!("   tamper → {:?}", r.kind);
    assert!(matches!(r.kind, ResultKind::AuthorityRefusal));

    // 3) Happy path under Activity + receipt + reverse
    let cap = host.mint("cap-ok", "kv.write", false, None);
    let good = Intent {
        action: "kv.write".into(),
        args: {
            let mut a = BTreeMap::new();
            a.insert("key".into(), json!("greeting"));
            a.insert("value".into(), json!("hello"));
            a
        },
        cap,
        trace: Some("trace-1".into()),
        idempotency_key: Some("idem-1".into()),
        activity_id: Some("act-demo".into()),
    };
    let r = host.submit(good);
    println!("\n3) kv.write → {:?} ops={:?}", r.kind, r.ops);
    println!("   digest={:?}", r.digest);
    let receipt = peer.apply(&r).unwrap();
    println!("   receipt landed={}", receipt.landed.len());
    host.report_receipt("act-demo", &receipt).expect("receipt");
    println!("   kv[greeting]={:?}", peer.kv_get("greeting"));

    // 4) Reverse (landed-informed)
    let rev = host.end_activity("act-demo").expect("reverse");
    println!(
        "\n4) end Activity → reverse ops={:?} used_landed={}",
        rev.ops, rev.used_landed
    );
    let rev_result = ResultMsg::ok(rev.ops);
    let _ = peer.apply(&rev_result);
    println!(
        "   kv[greeting] after reverse={:?}",
        peer.kv_get("greeting")
    );

    // 5) ui.morph + snapshot reverse
    let peer_ui = Peer::with_ui();
    let cap_ui = host.mint("cap-ui", "ui.morph", false, None);
    let r = host.submit(Intent {
        action: "ui.morph".into(),
        args: {
            let mut a = BTreeMap::new();
            a.insert("target".into(), json!("hdr"));
            a.insert("patch".into(), json!({"t": "hello"}));
            a.insert("snapshot".into(), json!({"t": ""}));
            a
        },
        cap: cap_ui,
        trace: None,
        idempotency_key: None,
        activity_id: Some("act-ui".into()),
    });
    println!("\n5) ui.morph → {:?} ops={}", r.kind, r.ops[0].fq());
    let rec = peer_ui.apply(&r).unwrap();
    host.report_receipt("act-ui", &rec).expect("ui receipt");
    println!("   ui[hdr]={:?}", peer_ui.ui_get("hdr"));
    let rev = host.end_activity("act-ui").expect("ui reverse");
    let _ = peer_ui.apply(&ResultMsg::ok(rev.ops));
    println!("   ui[hdr] after restore={:?}", peer_ui.ui_get("hdr"));

    // 6) kv.delete with prior → reverse restores
    let cap_del = host.mint("cap-del", "kv.delete", false, None);
    let _ = peer.apply(&host.submit(Intent {
        action: "kv.write".into(),
        args: {
            let mut a = BTreeMap::new();
            a.insert("key".into(), json!("note"));
            a.insert("value".into(), json!("keep"));
            a
        },
        cap: host.mint("cap-note", "kv.write", false, None),
        trace: None,
        idempotency_key: None,
        activity_id: None,
    }));
    let r = host.submit(Intent {
        action: "kv.delete".into(),
        args: {
            let mut a = BTreeMap::new();
            a.insert("key".into(), json!("note"));
            a.insert("prior".into(), json!("keep"));
            a
        },
        cap: cap_del,
        trace: None,
        idempotency_key: None,
        activity_id: Some("act-del".into()),
    });
    let rec = peer.apply(&r).unwrap();
    host.report_receipt("act-del", &rec).expect("del receipt");
    println!(
        "\n6) kv.delete(prior) → {:?} kv[note]={:?}",
        r.kind,
        peer.kv_get("note")
    );
    let rev = host.end_activity("act-del").expect("del reverse");
    let _ = peer.apply(&ResultMsg::ok(rev.ops));
    println!("   kv[note] after reverse={:?}", peer.kv_get("note"));

    // 7) Cap HMAC: unsigned refused when policy on
    let signed = Host::with_clock(1_000).with_hmac_key([0x0b; 32]);
    let cap = signed.mint("cap-hmac", "kv.write", false, None);
    let mut args = BTreeMap::new();
    args.insert("key".into(), json!("s"));
    args.insert("value".into(), json!(1));
    let ok = signed.submit(Intent {
        action: "kv.write".into(),
        args: args.clone(),
        cap: cap.clone(),
        trace: None,
        idempotency_key: None,
        activity_id: None,
    });
    let mut bare = cap;
    bare.sig = None;
    let no = signed.submit(Intent {
        action: "kv.write".into(),
        args,
        cap: bare,
        trace: None,
        idempotency_key: None,
        activity_id: None,
    });
    println!(
        "\n7) Cap HMAC signed={:?} unsigned={:?} ops_unsigned={}",
        ok.kind,
        no.kind,
        no.ops.len()
    );

    println!("\n=== demo ok ===");
}

fn make_peer(case: &VectorCase) -> Peer {
    match case.peer_profile.as_deref() {
        Some("ui") => Peer::with_ui(),
        _ => match case.peer_unknown_policy.as_deref() {
            Some("fail_batch") => Peer::with_policy(UnknownOpPolicy::FailBatch),
            _ => Peer::with_policy(UnknownOpPolicy::Skip),
        },
    }
}

fn parse_hmac_key(hex: &str) -> Result<[u8; 32], String> {
    let hex = hex.trim();
    if hex.len() != 64 {
        return Err(format!("hmac_key must be 64 hex chars, got {}", hex.len()));
    }
    let mut key = [0u8; 32];
    for i in 0..32 {
        key[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)
            .map_err(|e| format!("hmac_key hex: {e}"))?;
    }
    Ok(key)
}

fn run_one(case: &VectorCase) -> Result<(), String> {
    let mut host = Host::with_clock(case.now.unwrap_or(0));
    if let Some(ref hex) = case.hmac_key {
        host = host.with_hmac_key(parse_hmac_key(hex)?);
    }
    let peer = make_peer(case);

    if let Some(ref prior) = case.prior_intent {
        let mut prior = prior.clone();
        if case.sign_cap {
            prior.cap = host.attach_sig(prior.cap);
        }
        let r0 = host.submit(prior);
        if case.prior_must_ok && !matches!(r0.kind, ResultKind::Ok) {
            return Err(format!("prior_intent was {:?}, expected ok", r0.kind));
        }
    }
    if let Some(ref aid) = case.prior_end_activity {
        host.end_activity(aid).map_err(|e| e.to_string())?;
    }

    let result = if let Some(ref pr) = case.peer_result {
        pr.clone()
    } else {
        let mut intent = case
            .intent
            .clone()
            .ok_or_else(|| "no intent and no peer_result".to_string())?;
        if case.sign_cap {
            intent.cap = host.attach_sig(intent.cap);
        }
        host.submit(intent)
    };

    check_result(case, &result).map_err(|e| e.to_string())?;

    if let Some(expect_consumed) = case.expect_once_consumed {
        let cap_id = case
            .intent
            .as_ref()
            .map(|i| i.cap.id.as_str())
            .or_else(|| case.prior_intent.as_ref().map(|i| i.cap.id.as_str()))
            .ok_or_else(|| "expect_once_consumed needs an intent".to_string())?;
        let got = host.once_store().is_consumed(cap_id);
        if got != expect_consumed {
            return Err(format!(
                "once consumed want {expect_consumed} got {got} for {cap_id}"
            ));
        }
    }

    let mut receipt = None;
    if case.peer_apply || case.report_receipt {
        receipt = peer.apply(&result);
    }
    if case.report_receipt {
        let aid = case
            .end_activity
            .as_deref()
            .or_else(|| case.intent.as_ref().and_then(|i| i.activity_id.as_deref()))
            .ok_or_else(|| "report_receipt needs activity_id / end_activity".to_string())?;
        let rec = receipt
            .as_ref()
            .ok_or_else(|| "report_receipt: apply produced no receipt".to_string())?;
        host.report_receipt(aid, rec).map_err(|e| e.to_string())?;
    }
    if let Some(ref expect_kv) = case.expect_peer_kv {
        for (k, v) in expect_kv {
            let got = peer.kv_get(k);
            if v.is_null() {
                if got.is_some() {
                    return Err(format!("peer kv[{k}] should be absent, got {got:?}"));
                }
            } else if got.as_ref() != Some(v) {
                return Err(format!("peer kv[{k}] want {v} got {got:?}"));
            }
        }
    }
    if let Some(ref expect_ui) = case.expect_peer_ui {
        for (k, v) in expect_ui {
            let got = peer.ui_get(k);
            if v.is_null() {
                if got.is_some() {
                    return Err(format!("peer ui[{k}] should be absent, got {got:?}"));
                }
            } else if got.as_ref() != Some(v) {
                return Err(format!("peer ui[{k}] want {v} got {got:?}"));
            }
        }
    }

    if let Some(ref aid) = case.end_activity {
        let rev = host.end_activity(aid).map_err(|e| e.to_string())?;
        if let Some(ref expected) = case.expect_reverse_ops {
            if &rev.ops != expected {
                return Err(format!(
                    "reverse ops mismatch: want {expected:?} got {:?}",
                    rev.ops
                ));
            }
        }
        if let Some(want) = case.expect_used_landed {
            if rev.used_landed != want {
                return Err(format!("used_landed want {want} got {}", rev.used_landed));
            }
        }
        if case.end_activity_again && host.end_activity(aid).is_ok() {
            return Err("second end_activity unexpectedly succeeded".into());
        }
    }
    Ok(())
}

fn run_vectors(dir: &PathBuf) {
    println!("Running vectors in {}", dir.display());
    let mut failed = 0usize;
    let mut passed = 0usize;
    let mut families: BTreeMap<String, usize> = BTreeMap::new();
    let cases = match load_vector_dir(dir) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("cannot read dir: {e}");
            std::process::exit(1);
        }
    };
    for (path, case) in cases {
        match run_one(&case) {
            Ok(()) => {
                println!("PASS {}  [{}]", case.id, case.family);
                passed += 1;
                *families.entry(case.family.clone()).or_insert(0) += 1;
            }
            Err(e) => {
                eprintln!("FAIL {} ({}): {e}", case.id, path.display());
                failed += 1;
            }
        }
    }
    println!("\n{passed} passed, {failed} failed");
    if !families.is_empty() {
        let summary: Vec<String> = families.iter().map(|(f, n)| format!("{f}:{n}")).collect();
        println!("families  {}", summary.join("  "));
    }
    if failed > 0 {
        std::process::exit(1);
    }
}
