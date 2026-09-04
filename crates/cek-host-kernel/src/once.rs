//! Atomic once-Cap consume store (in-memory reference).
//!
//! ## Edge cases closed
//!
//! - **Check before claim**: `ensure_available` refuses if already consumed
//!   without recording a new claim (used before dispatch).
//! - **Commit after success**: `commit` records consumption only after
//!   successful dispatch so a dispatch failure does not burn a once-Cap.
//! - **Non-once Caps**: always available; commit is a no-op.
//!
//! Implements [`crate::OnceBackend`] so a durable backend can replace this.

use crate::{HostError, HostResult, OnceBackend};
use std::collections::HashSet;
use std::sync::Mutex;

/// Tracks consumed once-Cap ids (in-memory).
#[derive(Debug, Default)]
pub struct OnceStore {
    inner: Mutex<HashSet<String>>,
}

impl OnceStore {
    /// Empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Legacy single-step consume (tests that need atomic claim).
    pub fn try_consume(&self, cap_id: &str, once: bool) -> HostResult<()> {
        self.ensure_available(cap_id, once)?;
        self.commit(cap_id, once)
    }
}

impl OnceBackend for OnceStore {
    fn ensure_available(&self, cap_id: &str, once: bool) -> HostResult<()> {
        if !once {
            return Ok(());
        }
        let g = self.inner.lock().map_err(|_| HostError::OnceStoreDown)?;
        if g.contains(cap_id) {
            return Err(HostError::Authority(format!(
                "once Cap already consumed: {cap_id}"
            )));
        }
        Ok(())
    }

    fn commit(&self, cap_id: &str, once: bool) -> HostResult<()> {
        if !once {
            return Ok(());
        }
        let mut g = self.inner.lock().map_err(|_| HostError::OnceStoreDown)?;
        if !g.insert(cap_id.to_string()) {
            // Race: another commit won — treat as authority failure.
            return Err(HostError::Authority(format!(
                "once Cap already consumed: {cap_id}"
            )));
        }
        Ok(())
    }

    fn is_consumed(&self, cap_id: &str) -> bool {
        self.inner
            .lock()
            .map(|g| g.contains(cap_id))
            .unwrap_or(false)
    }
}
