//! Baseline Op world state (reference in-memory).

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use serde_json::Value;
use std::collections::HashMap;

/// Simple key-value store for `kv.*` Ops.
#[derive(Debug, Default, Clone)]
pub struct KvStore {
    map: HashMap<String, Value>,
}

impl KvStore {
    /// Empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set key.
    pub fn set(&mut self, key: impl Into<String>, value: Value) {
        self.map.insert(key.into(), value);
    }

    /// Delete key.
    pub fn delete(&mut self, key: &str) {
        self.map.remove(key);
    }

    /// Get key.
    pub fn get(&self, key: &str) -> Option<Value> {
        self.map.get(key).cloned()
    }

    /// Number of live keys (tests / diagnostics).
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// True if no keys.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn set_get_delete() {
        let mut kv = KvStore::new();
        assert!(kv.is_empty());
        kv.set("a", json!(1));
        assert_eq!(kv.get("a"), Some(json!(1)));
        assert_eq!(kv.len(), 1);
        kv.set("a", json!(2));
        assert_eq!(kv.get("a"), Some(json!(2)));
        kv.delete("a");
        assert!(kv.get("a").is_none());
        assert!(kv.is_empty());
        kv.delete("missing");
        assert!(kv.get("missing").is_none());
    }
}
