//! Domain `ui.*` Ops — L5 pack, not Baseline.
//!
//! Host projects `ui.morph` → `ui.dom.morph`. Reverse is `ui.dom.restore`
//! when a snapshot was carried in the morph payload (honest snapshot class).
//! Baseline Peers skip these Ops (unknown); [`lower_to_baseline`] is the
//! optional classic projection.

use crate::{baseline, Op};
use serde_json::{json, Value};

/// Fully-qualified UI domain Ops.
pub const UI_OPS: &[&str] = &["ui.dom.morph", "ui.dom.restore"];

/// True if `ns.name` is a UI domain Op.
pub fn is_ui(ns: &str, name: &str) -> bool {
    UI_OPS.contains(&format!("{ns}.{name}").as_str())
}

/// Build `ui.dom.morph`. Snapshot lives **on the Op** so landed-first reverse
/// can restore without consulting Intent args.
pub fn ui_morph(target: impl Into<String>, patch: Value, snapshot: Option<Value>) -> Op {
    let mut payload = json!({
        "target": target.into(),
        "patch": patch,
    });
    if let Some(snap) = snapshot {
        payload
            .as_object_mut()
            .expect("object")
            .insert("snapshot".into(), snap);
    }
    Op {
        ns: "ui.dom".into(),
        name: "morph".into(),
        payload,
    }
}

/// Build `ui.dom.restore` (snapshot reverse of a morph).
pub fn ui_restore(target: impl Into<String>, snapshot: Value) -> Op {
    Op {
        ns: "ui.dom".into(),
        name: "restore".into(),
        payload: json!({
            "target": target.into(),
            "snapshot": snapshot,
        }),
    }
}

/// Lower a UI (or already-Baseline) Op to a classic Baseline Op.
///
/// `ui.dom.morph` / `ui.dom.restore` become `kv.set` under `ui:{target}`.
/// Unknown Ops return `None`.
pub fn lower_to_baseline(op: &Op) -> Option<Op> {
    if baseline::is_baseline(&op.ns, &op.name) {
        return Some(op.clone());
    }
    if op.ns == "ui.dom" && (op.name == "morph" || op.name == "restore") {
        let target = op.payload.get("target").and_then(|v| v.as_str())?;
        let value = if op.name == "morph" {
            op.payload.get("patch").cloned().unwrap_or(Value::Null)
        } else {
            op.payload.get("snapshot").cloned().unwrap_or(Value::Null)
        };
        return Some(baseline::kv_set(format!("ui:{target}"), value));
    }
    None
}

/// Inverse of a UI morph when a snapshot is present.
pub fn inverse_ui(op: &Op) -> Option<Op> {
    if op.ns == "ui.dom" && op.name == "morph" {
        let target = op.payload.get("target").and_then(|v| v.as_str())?;
        let snap = op.payload.get("snapshot").cloned()?;
        return Some(ui_restore(target, snap));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn morph_restore_round_names() {
        let m = ui_morph("hdr", json!({"t": 1}), Some(json!({"t": 0})));
        assert_eq!(m.fq(), "ui.dom.morph");
        assert!(is_ui("ui.dom", "morph"));
        let r = inverse_ui(&m).unwrap();
        assert_eq!(r.fq(), "ui.dom.restore");
        let low = lower_to_baseline(&m).unwrap();
        assert_eq!(low.fq(), "kv.set");
        assert_eq!(
            low.payload.get("key").and_then(|v| v.as_str()),
            Some("ui:hdr")
        );
    }

    #[test]
    fn morph_without_snapshot_has_no_inverse() {
        let m = ui_morph("x", json!(1), None);
        assert!(inverse_ui(&m).is_none());
    }
}
