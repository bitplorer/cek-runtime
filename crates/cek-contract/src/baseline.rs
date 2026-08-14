//! Baseline Ops — permanent classic catalog.

use crate::Op;
use serde_json::json;

/// Fully-qualified Baseline Op names every correct Peer can aim at.
pub const BASELINE_OPS: &[&str] = &["kv.set", "kv.delete", "log.append"];

/// True if `ns.name` is in the Baseline catalog.
pub fn is_baseline(ns: &str, name: &str) -> bool {
    let fq = format!("{ns}.{name}");
    BASELINE_OPS.contains(&fq.as_str())
}

/// Build a `kv.set` Op.
pub fn kv_set(key: impl Into<String>, value: serde_json::Value) -> Op {
    Op {
        ns: "kv".into(),
        name: "set".into(),
        payload: json!({ "key": key.into(), "value": value }),
    }
}

/// Build a `kv.delete` Op.
pub fn kv_delete(key: impl Into<String>) -> Op {
    Op {
        ns: "kv".into(),
        name: "delete".into(),
        payload: json!({ "key": key.into() }),
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
