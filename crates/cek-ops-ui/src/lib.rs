//! Peer **UI/DOM driver** — flat [`UiStore`] and tree [`DomTree`].
//!
//! Not a kernel. Not a browser. The Host projects `ui.morph` → `ui.dom.morph`;
//! this crate only applies those Ops. Catalog: repo `DRIVERS.md`.

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

    /// Drop a target.
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

/// Tree-shaped document. Nodes: `{ tag, attrs, children, text? }`.
///
/// Address a node by `attrs.id`, `#id`, or a `/index/index` path from the forest.
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

    /// Replace the addressed node. `false` if missing.
    pub fn morph(&mut self, target: &str, patch: Value) -> bool {
        replace_at(&mut self.roots, target, patch)
    }

    /// Write snapshot back.
    pub fn restore(&mut self, target: &str, snapshot: Value) -> bool {
        self.morph(target, snapshot)
    }

    /// Find a node by id, `#id`, or `/path`.
    pub fn get(&self, target: &str) -> Option<Value> {
        find_at(&self.roots, target).cloned()
    }

    /// Insert `child` under parent (id or path). `false` if parent missing.
    pub fn insert_child(&mut self, parent: &str, child: Value) -> bool {
        if let Some(n) = find_at_mut(&mut self.roots, parent) {
            if let Some(kids) = n.get_mut("children").and_then(|c| c.as_array_mut()) {
                kids.push(child);
                return true;
            }
            if let Value::Object(map) = n {
                map.insert("children".into(), json!([child]));
                return true;
            }
        }
        false
    }

    /// Remove the addressed node. Returns the removed snapshot.
    pub fn remove(&mut self, target: &str) -> Option<Value> {
        take_at(&mut self.roots, target)
    }

    /// Set text on a node.
    pub fn set_text(&mut self, target: &str, text: impl Into<String>) -> bool {
        match find_at_mut(&mut self.roots, target) {
            Some(Value::Object(map)) => {
                map.insert("text".into(), Value::String(text.into()));
                true
            }
            _ => false,
        }
    }

    /// Merge attrs onto a node.
    pub fn set_attr(&mut self, target: &str, key: &str, value: Value) -> bool {
        if let Some(n) = find_at_mut(&mut self.roots, target) {
            if let Some(Value::Object(attrs)) = n.get_mut("attrs") {
                attrs.insert(key.into(), value);
                return true;
            }
        }
        false
    }

    /// Deterministic id → node map.
    pub fn by_id(&self) -> BTreeMap<String, Value> {
        let mut out = BTreeMap::new();
        collect_ids(&self.roots, &mut out);
        out
    }

    /// HTML-ish render (for tests / JS runtime). Not a browser.
    pub fn html(&self) -> String {
        self.roots.iter().map(render).collect()
    }

    /// Root count.
    pub fn len(&self) -> usize {
        self.roots.len()
    }

    /// Empty forest?
    pub fn is_empty(&self) -> bool {
        self.roots.is_empty()
    }
}

/// Build a DOM node object.
pub fn dom_node(id: &str, tag: &str, mut attrs: Value, children: Vec<Value>) -> Value {
    if let Value::Object(map) = &mut attrs {
        map.insert("id".into(), Value::String(id.into()));
    }
    json!({ "tag": tag, "attrs": attrs, "children": children })
}

fn norm(target: &str) -> &str {
    target.strip_prefix('#').unwrap_or(target)
}

fn replace_at(nodes: &mut [Value], target: &str, patch: Value) -> bool {
    if let Some(path) = parse_path(target) {
        return replace_path(nodes, &path, patch);
    }
    let id = norm(target);
    for n in nodes.iter_mut() {
        if id_of(n) == Some(id) {
            *n = patch;
            return true;
        }
        if let Some(kids) = n.get_mut("children").and_then(|c| c.as_array_mut()) {
            if replace_at(kids, target, patch.clone()) {
                return true;
            }
        }
    }
    false
}

fn find_at<'a>(nodes: &'a [Value], target: &str) -> Option<&'a Value> {
    if let Some(path) = parse_path(target) {
        return get_path(nodes, &path);
    }
    let id = norm(target);
    for n in nodes {
        if id_of(n) == Some(id) {
            return Some(n);
        }
        if let Some(kids) = n.get("children").and_then(|c| c.as_array()) {
            if let Some(hit) = find_at(kids, target) {
                return Some(hit);
            }
        }
    }
    None
}

fn find_at_mut<'a>(nodes: &'a mut [Value], target: &str) -> Option<&'a mut Value> {
    if parse_path(target).is_some() {
        let path = parse_path(target).unwrap();
        return get_path_mut(nodes, &path);
    }
    let id = norm(target).to_string();
    for n in nodes.iter_mut() {
        if id_of(n) == Some(id.as_str()) {
            return Some(n);
        }
        if n.get("children").and_then(|c| c.as_array()).is_some() {
            if let Some(kids) = n.get_mut("children").and_then(|c| c.as_array_mut()) {
                if let Some(hit) = find_at_mut(kids, &id) {
                    return Some(hit);
                }
            }
        }
    }
    None
}

fn take_at(nodes: &mut Vec<Value>, target: &str) -> Option<Value> {
    if let Some(path) = parse_path(target) {
        return take_path(nodes, &path);
    }
    let id = norm(target);
    if let Some(i) = nodes.iter().position(|n| id_of(n) == Some(id)) {
        return Some(nodes.remove(i));
    }
    for n in nodes.iter_mut() {
        if let Some(kids) = n.get_mut("children").and_then(|c| c.as_array_mut()) {
            if let Some(hit) = take_at(kids, target) {
                return Some(hit);
            }
        }
    }
    None
}

fn parse_path(target: &str) -> Option<Vec<usize>> {
    let t = target.strip_prefix('/')?;
    if t.is_empty() {
        return Some(vec![]);
    }
    t.split('/')
        .map(|s| s.parse::<usize>().ok())
        .collect::<Option<Vec<_>>>()
}

fn get_path<'a>(nodes: &'a [Value], path: &[usize]) -> Option<&'a Value> {
    match path {
        [] => None,
        [i] => nodes.get(*i),
        [i, rest @ ..] => {
            let kids = nodes.get(*i)?.get("children")?.as_array()?;
            get_path(kids, rest)
        }
    }
}

fn get_path_mut<'a>(nodes: &'a mut [Value], path: &[usize]) -> Option<&'a mut Value> {
    match path {
        [] => None,
        [i] => nodes.get_mut(*i),
        [i, rest @ ..] => {
            let kids = nodes.get_mut(*i)?.get_mut("children")?.as_array_mut()?;
            get_path_mut(kids, rest)
        }
    }
}

fn replace_path(nodes: &mut [Value], path: &[usize], patch: Value) -> bool {
    if let Some(n) = get_path_mut(nodes, path) {
        *n = patch;
        return true;
    }
    false
}

fn take_path(nodes: &mut Vec<Value>, path: &[usize]) -> Option<Value> {
    match path {
        [] => None,
        [i] if *i < nodes.len() => Some(nodes.remove(*i)),
        [i, rest @ ..] => {
            let kids = nodes.get_mut(*i)?.get_mut("children")?.as_array_mut()?;
            take_path(kids, rest)
        }
    }
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

fn render(n: &Value) -> String {
    let tag = n.get("tag").and_then(|t| t.as_str()).unwrap_or("div");
    let mut attrs = String::new();
    if let Some(Value::Object(map)) = n.get("attrs") {
        let mut keys: Vec<_> = map.keys().collect();
        keys.sort();
        for k in keys {
            if let Some(v) = map.get(k).and_then(|v| v.as_str()) {
                attrs.push_str(&format!(" {k}=\"{v}\""));
            }
        }
    }
    let text = n.get("text").and_then(|t| t.as_str()).unwrap_or("");
    let kids = n
        .get("children")
        .and_then(|c| c.as_array())
        .map(|a| a.iter().map(render).collect::<String>())
        .unwrap_or_default();
    format!("<{tag}{attrs}>{text}{kids}</{tag}>")
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
        ui.restore("hdr", json!({"t": "old"}));
        assert_eq!(ui.get("hdr"), Some(json!({"t": "old"})));
    }

    #[test]
    fn dom_tree_morph_restore() {
        let mut d = DomTree::with_root();
        let h1 = dom_node("root", "h1", json!({"id": "root"}), vec![]);
        assert!(d.morph("#root", h1));
        assert_eq!(d.get("root").unwrap()["tag"], "h1");
        let div = dom_node("root", "div", json!({"id": "root"}), vec![]);
        assert!(d.restore("/0", div));
        assert_eq!(d.get("/0").unwrap()["tag"], "div");
    }

    #[test]
    fn apply_op_tree_unknown_id_fails() {
        let mut d = DomTree::with_root();
        let op = cek_contract::ui_morph("missing", json!({"tag": "p"}), None);
        assert!(apply_op_tree(&mut d, &op).is_err());
    }

    #[test]
    fn insert_child_set_text_html() {
        let mut d = DomTree::with_root();
        let kid = dom_node("title", "h1", json!({"id": "title"}), vec![]);
        assert!(d.insert_child("root", kid));
        assert!(d.set_text("title", "Hello"));
        assert!(d.set_attr("title", "class", json!("hero")));
        let html = d.html();
        assert!(html.contains("<h1 class=\"hero\" id=\"title\">Hello</h1>"), "{html}");
    }

    #[test]
    fn remove_then_missing() {
        let mut d = DomTree::with_root();
        let snap = d.remove("root").unwrap();
        assert_eq!(snap["tag"], "div");
        assert!(d.get("root").is_none());
        assert!(d.is_empty());
    }
}
