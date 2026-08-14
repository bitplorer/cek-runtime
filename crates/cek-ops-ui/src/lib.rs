//! In-memory UI target store for `ui.dom.morph` / `ui.dom.restore`.
//!
//! This is a **reference DOM**: a map of target id → JSON node. It is not a
//! browser. Snapshot reverse writes the saved node back.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use serde_json::Value;
use std::collections::HashMap;

/// World of named UI targets.
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn morph_then_restore() {
        let mut ui = UiStore::new();
        assert!(ui.is_empty());
        ui.morph("hdr", json!({"t": "new"}));
        assert_eq!(ui.get("hdr"), Some(json!({"t": "new"})));
        ui.restore("hdr", json!({"t": "old"}));
        assert_eq!(ui.get("hdr"), Some(json!({"t": "old"})));
        assert_eq!(ui.len(), 1);
        ui.clear("hdr");
        assert!(ui.is_empty());
    }
}
