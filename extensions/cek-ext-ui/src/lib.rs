//! UI domain pack — **extension**, not kernel.
//!
//! Projects `ui.morph` / `ui.restore`. Does not mint Caps.
//! [`DomTree`] is an optional tree-shaped world; the kernel never sees it.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use cek_contract::{ui_morph, ui_restore, Intent, Op, ACTION_UI_MORPH, ACTION_UI_RESTORE};
use cek_host_kernel::DomainPack;
use serde_json::Value;
use std::collections::BTreeMap;

/// Host pack for `ui.*` Actions.
#[derive(Debug, Default, Clone, Copy)]
pub struct UiPack;

impl DomainPack for UiPack {
    fn name(&self) -> &'static str {
        "ui"
    }

    fn project(&self, intent: &Intent) -> Option<Result<Vec<Op>, String>> {
        match intent.action.as_str() {
            ACTION_UI_MORPH => Some(project_morph(intent)),
            ACTION_UI_RESTORE => Some(project_restore(intent)),
            _ => None,
        }
    }

    fn inverse(&self, op: &Op) -> Option<Op> {
        cek_contract::inverse_ui(op)
    }
}

fn project_morph(intent: &Intent) -> Result<Vec<Op>, String> {
    let target = intent
        .args
        .get("target")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "ui.morph requires string args.target".to_string())?;
    if target.is_empty() {
        return Err("ui.morph target must be non-empty".into());
    }
    let patch = intent
        .args
        .get("patch")
        .cloned()
        .ok_or_else(|| "ui.morph requires args.patch".to_string())?;
    let snapshot = intent.args.get("snapshot").cloned();
    Ok(vec![ui_morph(target, patch, snapshot)])
}

fn project_restore(intent: &Intent) -> Result<Vec<Op>, String> {
    let target = intent
        .args
        .get("target")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "ui.restore requires string args.target".to_string())?;
    if target.is_empty() {
        return Err("ui.restore target must be non-empty".into());
    }
    let snapshot = intent
        .args
        .get("snapshot")
        .cloned()
        .ok_or_else(|| "ui.restore requires args.snapshot".to_string())?;
    Ok(vec![ui_restore(target, snapshot)])
}

/// Optional tree-shaped UI world (extension). Not a browser.
///
/// Nodes are `{ "tag", "attrs", "children" }`. Morph replaces a node by id
/// (the target is an id attribute). Restore writes the snapshot node back.
#[derive(Debug, Default, Clone)]
pub struct DomTree {
    /// Forest of root nodes (JSON objects).
    roots: Vec<Value>,
}

impl DomTree {
    /// Empty forest.
    pub fn new() -> Self {
        Self::default()
    }

    /// Document with a single empty `root` node.
    pub fn with_root() -> Self {
        Self {
            roots: vec![node(
                "root",
                "div",
                Value::Object(Default::default()),
                vec![],
            )],
        }
    }

    /// Replace the node whose `attrs.id` equals `target`.
    pub fn morph(&mut self, target: &str, patch: Value) -> bool {
        walk_replace(&mut self.roots, target, patch)
    }

    /// Write snapshot back onto `target`.
    pub fn restore(&mut self, target: &str, snapshot: Value) -> bool {
        self.morph(target, snapshot)
    }

    /// Find a node by id.
    pub fn get(&self, target: &str) -> Option<Value> {
        find_id(&self.roots, target).cloned()
    }

    /// Deterministic snapshot of id → node.
    pub fn by_id(&self) -> BTreeMap<String, Value> {
        let mut out = BTreeMap::new();
        collect_ids(&self.roots, &mut out);
        out
    }
}

fn node(id: &str, tag: &str, mut attrs: Value, children: Vec<Value>) -> Value {
    if let Value::Object(map) = &mut attrs {
        map.insert("id".into(), Value::String(id.into()));
    }
    serde_json::json!({
        "tag": tag,
        "attrs": attrs,
        "children": children,
    })
}

fn walk_replace(nodes: &mut [Value], target: &str, patch: Value) -> bool {
    for n in nodes.iter_mut() {
        if id_of(n) == Some(target) {
            *n = patch;
            return true;
        }
        if let Some(kids) = n.get_mut("children").and_then(|c| c.as_array_mut()) {
            if walk_replace(kids, target, patch.clone()) {
                return true;
            }
        }
    }
    false
}

fn find_id<'a>(nodes: &'a [Value], target: &str) -> Option<&'a Value> {
    for n in nodes {
        if id_of(n) == Some(target) {
            return Some(n);
        }
        if let Some(kids) = n.get("children").and_then(|c| c.as_array()) {
            if let Some(hit) = find_id(kids, target) {
                return Some(hit);
            }
        }
    }
    None
}

fn collect_ids(nodes: &[Value], out: &mut BTreeMap<String, Value>) {
    for n in nodes {
        if let Some(id) = id_of(n) {
            out.insert(id.to_string(), n.clone());
        }
        if let Some(kids) = n.get("children").and_then(|c| c.as_array()) {
            collect_ids(kids, out);
        }
    }
}

fn id_of(n: &Value) -> Option<&str> {
    n.get("attrs")
        .and_then(|a| a.get("id"))
        .and_then(|v| v.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cek_host_kernel::Host;
    use serde_json::json;
    use std::sync::Arc;

    #[test]
    fn pack_projects_morph() {
        let host = Host::with_clock(1).with_pack(Arc::new(UiPack));
        let cap = host.mint("c", "ui.morph", false, None);
        let mut args = std::collections::BTreeMap::new();
        args.insert("target".into(), json!("hdr"));
        args.insert("patch".into(), json!({"t": "x"}));
        args.insert("snapshot".into(), json!({"t": ""}));
        let r = host.submit(cek_contract::Intent {
            action: "ui.morph".into(),
            args,
            cap,
            trace: None,
            idempotency_key: None,
            activity_id: Some("act-ui".into()),
        });
        assert!(matches!(r.kind, cek_contract::ResultKind::Ok));
        assert_eq!(r.ops[0].fq(), "ui.dom.morph");
        let rev = host.end_activity("act-ui").unwrap();
        assert_eq!(rev.ops[0].fq(), "ui.dom.restore");
    }

    #[test]
    fn morph_without_snapshot_is_non_reversible() {
        let host = Host::with_clock(1).with_pack(Arc::new(UiPack));
        let cap = host.mint("c2", "ui.morph", false, None);
        let mut args = std::collections::BTreeMap::new();
        args.insert("target".into(), json!("hdr"));
        args.insert("patch".into(), json!({"t": 1}));
        let r = host.submit(cek_contract::Intent {
            action: "ui.morph".into(),
            args,
            cap,
            trace: None,
            idempotency_key: None,
            activity_id: Some("act-ui".into()),
        });
        assert!(matches!(r.kind, cek_contract::ResultKind::Ok));
        let rev = host.end_activity("act-ui").unwrap();
        assert!(rev.ops.is_empty());
        assert!(!rev.non_reversible.is_empty());
    }

    #[test]
    fn kernel_without_pack_does_not_project_ui() {
        let host = Host::with_clock(1);
        let cap = host.mint("c", "ui.morph", false, None);
        let mut args = std::collections::BTreeMap::new();
        args.insert("target".into(), json!("hdr"));
        args.insert("patch".into(), json!({"t": "x"}));
        let r = host.submit(cek_contract::Intent {
            action: "ui.morph".into(),
            args,
            cap,
            trace: None,
            idempotency_key: None,
            activity_id: None,
        });
        assert!(matches!(r.kind, cek_contract::ResultKind::DispatchError));
        assert!(r.ops.is_empty());
    }

    #[test]
    fn dom_tree_morph_restore() {
        let mut d = DomTree::with_root();
        assert_eq!(d.get("root").unwrap()["tag"], "div");
        d.morph("root", node("root", "h1", json!({"id": "root"}), vec![]));
        assert_eq!(d.get("root").unwrap()["tag"], "h1");
        d.restore("root", node("root", "div", json!({"id": "root"}), vec![]));
        assert_eq!(d.get("root").unwrap()["tag"], "div");
    }
}
