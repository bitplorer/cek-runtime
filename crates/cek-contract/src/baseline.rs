//! Baseline Ops — permanent classic catalog.
//!
//! Legality is a **pair** `(ns, name)`, not the concatenated FQ string.
//! `name` is a single token (no dots).

use crate::Op;
use serde_json::json;

/// Fully-qualified Baseline Op names every correct Peer can aim at.
pub const BASELINE_OPS: &[&str] = &["kv.set", "kv.delete", "log.append"];

/// Declared Baseline `(ns, name)` pairs. Source of `is_baseline`.
pub const BASELINE_PAIRS: &[(&str, &str)] = &[("kv", "set"), ("kv", "delete"), ("log", "append")];

/// True if `(ns, name)` is a declared Baseline pair.
pub fn is_baseline(ns: &str, name: &str) -> bool {
    BASELINE_PAIRS.iter().any(|(n, m)| *n == ns && *m == name)
}

/// Build a `kv.set` Op.
pub fn kv_set(key: impl Into<String>, value: serde_json::Value) -> Op {
    Op {
        ns: "kv".into(),
        name: "set".into(),
        payload: json!({ "key": key.into(), "value": value }),
    }
}

/// Build a `kv.delete` Op. Optional `prior` is the value to restore on reverse.
pub fn kv_delete(key: impl Into<String>) -> Op {
    kv_delete_prior(key, None)
}

/// `kv.delete` with a prior-value snapshot on the Op (landed-first reverse).
pub fn kv_delete_prior(key: impl Into<String>, prior: Option<serde_json::Value>) -> Op {
    let mut payload = json!({ "key": key.into() });
    if let Some(p) = prior {
        payload
            .as_object_mut()
            .expect("object")
            .insert("prior".into(), p);
    }
    Op {
        ns: "kv".into(),
        name: "delete".into(),
        payload,
    }
}

/// Build a `log.append` Op.
pub fn log_append(message: impl Into<String>) -> Op {
    Op {
        ns: "log".into(),
        name: "append".into(),
        payload: json!({ "message": message.into() }),
    }
}

/// Inverse of `kv.set` when prior value is known; else delete key.
pub fn kv_set_inverse(key: impl Into<String>, prior: Option<serde_json::Value>) -> Op {
    match prior {
        Some(v) => kv_set(key, v),
        None => kv_delete(key),
    }
}

/// Inverse of a Baseline kv Op when enough payload is present.
///
/// - `kv.set` → `kv.delete` (no prior needed to undo a set)
/// - `kv.delete` with `prior` → `kv.set` of that prior
/// - `kv.delete` without prior / `log.append` → `None` (honest non-reversible)
pub fn inverse_kv(op: &Op) -> Option<Op> {
    if op.ns != "kv" {
        return None;
    }
    match op.name.as_str() {
        "set" => {
            let key = op.payload.get("key").and_then(|v| v.as_str())?;
            Some(kv_delete(key))
        }
        "delete" => {
            let key = op.payload.get("key").and_then(|v| v.as_str())?;
            let prior = op.payload.get("prior").cloned()?;
            Some(kv_set(key, prior))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn delete_prior_inverts_to_set() {
        let d = kv_delete_prior("k", Some(json!(1)));
        let inv = inverse_kv(&d).unwrap();
        assert_eq!(inv.fq(), "kv.set");
        assert_eq!(inv.payload.get("value"), Some(&json!(1)));
        assert!(inverse_kv(&kv_delete("k")).is_none());
    }

    #[test]
    fn set_inverts_to_delete() {
        let inv = inverse_kv(&kv_set("k", json!(1))).unwrap();
        assert_eq!(inv.fq(), "kv.delete");
        assert!(inv.payload.get("prior").is_none());
    }

    #[test]
    fn pair_not_concat() {
        assert!(is_baseline("kv", "set"));
        assert!(!is_baseline("k", "v.set"));
        assert!(!is_baseline("kv.set", ""));
        assert_eq!(BASELINE_PAIRS.len(), BASELINE_OPS.len());
    }
}
