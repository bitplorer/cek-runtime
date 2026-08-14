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
}
