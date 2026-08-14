//! Peer **drivers** for UI: flat target map and a tree-shaped DOM.
//!
//! Not a kernel. Not a browser. Morph/restore are data.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};

/// World of named UI targets (flat map).
#[derive(Debug, Default, Clone)]
pub struct UiStore {
    nodes: HashMap<String, Value>,
}

impl UiStore {
    /// Empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace `target` with `patch` (morph).
    pub fn morph(&mut self, target: impl Into<String>, patch: Value) {
        self.nodes.insert(target.into(), patch);
    }

    /// Write `snapshot` back onto `target` (restore).
    pub fn restore(&mut self, target: impl Into<String>, snapshot: Value) {
        self.nodes.insert(target.into(), snapshot);
    }

    /// Current node, if any.
    pub fn get(&self, target: &str) -> Option<Value> {
        self.nodes.get(target).cloned()
    }

    /// Drop a target (tests / diagnostics).
    pub fn clear(&mut self, target: &str) {
        self.nodes.remove(target);
    }

    /// Live target count.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// True if no targets.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Snapshot of live targets (deterministic order).
    pub fn snapshot(&self) -> BTreeMap<String, Value> {
        self.nodes
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }
}

/// Tree-shaped document. Nodes are `{ tag, attrs, children }`.
/// Target is `attrs.id`. Morph replaces that node. Restore writes the snapshot back.
#[derive(Debug, Default, Clone)]
pub struct DomTree {
    roots: Vec<Value>,
}

impl DomTree {
    /// Empty forest.
    pub fn new() -> Self {
        Self::default()
    }

    /// One `div#root`.
    pub fn with_root() -> Self {
        Self {
            roots: vec![dom_node("root", "div", json!({"id": "root"}), vec![])],
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

    /// Deterministic id → node map.
    pub fn by_id(&self) -> BTreeMap<String, Value> {
        let mut out = BTreeMap::new();
        collect_ids(&self.roots, &mut out);
        out
    }
}

/// Build a DOM node object.
pub fn dom_node(id: &str, tag: &str, mut attrs: Value, children: Vec<Value>) -> Value {
    if let Value::Object(map) = &mut attrs {
        map.insert("id".into(), Value::String(id.into()));
    }
    json!({ "tag": tag, "attrs": attrs, "children": children })
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

/// Apply one `ui.dom.*` Op onto a flat store.
#[allow(clippy::result_unit_err)]
pub fn apply_op(store: &mut UiStore, op: &cek_contract::Op) -> Result<(), ()> {
    match (op.ns.as_str(), op.name.as_str()) {
        ("ui.dom", "morph") => {
            let target = op.payload.get("target").and_then(|v| v.as_str()).ok_or(())?;
            let patch = op.payload.get("patch").cloned().ok_or(())?;
            store.morph(target, patch);
            Ok(())
        }
        ("ui.dom", "restore") => {
            let target = op.payload.get("target").and_then(|v| v.as_str()).ok_or(())?;
            let snapshot = op.payload.get("snapshot").cloned().ok_or(())?;
            store.restore(target, snapshot);
            Ok(())
        }
        _ => Err(()),
    }
}

/// Apply one `ui.dom.*` Op onto a tree document.
#[allow(clippy::result_unit_err)]
pub fn apply_op_tree(dom: &mut DomTree, op: &cek_contract::Op) -> Result<(), ()> {
    match (op.ns.as_str(), op.name.as_str()) {
        ("ui.dom", "morph") => {
            let target = op.payload.get("target").and_then(|v| v.as_str()).ok_or(())?;
            let patch = op.payload.get("patch").cloned().ok_or(())?;
            if dom.morph(target, patch) {
                Ok(())
            } else {
                Err(())
            }
        }
        ("ui.dom", "restore") => {
            let target = op.payload.get("target").and_then(|v| v.as_str()).ok_or(())?;
            let snapshot = op.payload.get("snapshot").cloned().ok_or(())?;
            if dom.restore(target, snapshot) {
                Ok(())
            } else {
                Err(())
            }
        }
        _ => Err(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn morph_then_restore() {
        let mut ui = UiStore::new();
        ui.morph("hdr", json!({"t": "new"}));
        assert_eq!(ui.get("hdr"), Some(json!({"t": "new"})));
        ui.restore("hdr", json!({"t": "old"}));
        assert_eq!(ui.get("hdr"), Some(json!({"t": "old"})));
    }

    #[test]
    fn dom_tree_morph_restore() {
        let mut d = DomTree::with_root();
        assert_eq!(d.get("root").unwrap()["tag"], "div");
        let h1 = dom_node("root", "h1", json!({"id": "root"}), vec![]);
        assert!(d.morph("root", h1));
        assert_eq!(d.get("root").unwrap()["tag"], "h1");
        let div = dom_node("root", "div", json!({"id": "root"}), vec![]);
        assert!(d.restore("root", div));
        assert_eq!(d.get("root").unwrap()["tag"], "div");
    }

    #[test]
    fn apply_op_tree_unknown_id_fails() {
        let mut d = DomTree::with_root();
        let op = cek_contract::ui_morph("missing", json!({"tag": "p"}), None);
        assert!(apply_op_tree(&mut d, &op).is_err());
    }
}
