//! Atomic once-Cap consume store (in-memory reference).

use crate::{HostError, HostResult};
use std::collections::HashSet;
use std::sync::Mutex;

/// Tracks consumed once-Cap ids.
#[derive(Debug, Default)]
pub struct OnceStore {
    inner: Mutex<HashSet<String>>,
}

impl OnceStore {
    /// Empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Try to consume a once Cap. Returns Ok if first use, Err if already used.
    ///
    /// Non-once Caps are no-ops (always Ok).
    pub fn try_consume(&self, cap_id: &str, once: bool) -> HostResult<()> {
        if !once {
            return Ok(());
        }
        let mut g = self
            .inner
            .lock()
            .map_err(|_| HostError::OnceStoreDown)?;
        if !g.insert(cap_id.to_string()) {
            return Err(HostError::Authority(format!(
                "once Cap already consumed: {cap_id}"
            )));
        }
        Ok(())
    }

    /// Test helper: whether id is marked consumed.
    pub fn is_consumed(&self, cap_id: &str) -> bool {
        self.inner
            .lock()
            .map(|g| g.contains(cap_id))
            .unwrap_or(false)
    }
}
