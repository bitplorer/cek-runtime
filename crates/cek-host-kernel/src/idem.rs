//! Idempotency bind store — same key returns same Result.
//!
//! Implements [`crate::IdemBackend`] so a durable backend can replace this.

use crate::{HostError, HostResult, IdemBackend};
use cek_contract::ResultMsg;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Clone, Debug)]
struct IdemEntry {
    digest: String,
    result: ResultMsg,
}

/// Maps idempotency key → prior Result (digest-bound). In-memory reference.
#[derive(Debug, Default)]
pub struct IdemStore {
    inner: Mutex<HashMap<String, IdemEntry>>,
}

impl IdemStore {
    /// Empty store.
    pub fn new() -> Self {
        Self::default()
    }
}

impl IdemBackend for IdemStore {
    fn get(&self, key: &str) -> HostResult<Option<ResultMsg>> {
        let g = self.inner.lock().map_err(|_| HostError::IdemStoreDown)?;
        Ok(g.get(key).map(|e| e.result.clone()))
    }

    fn put_or_check(&self, key: &str, digest: &str, result: &ResultMsg) -> HostResult<IdemOutcome> {
        let mut g = self.inner.lock().map_err(|_| HostError::IdemStoreDown)?;
        match g.get(key) {
            None => {
                g.insert(
                    key.to_string(),
                    IdemEntry {
                        digest: digest.to_string(),
                        result: result.clone(),
                    },
                );
                Ok(IdemOutcome::Recorded)
            }
            Some(prev) if prev.digest == digest => Ok(IdemOutcome::ReplaySame {
                result: prev.result.clone(),
            }),
            Some(_) => Err(HostError::Authority(format!(
                "idempotency conflict for key `{key}`"
            ))),
        }
    }
}

/// Outcome of recording an idempotency bind.
#[derive(Debug, Clone)]
pub enum IdemOutcome {
    /// First time this key was seen.
    Recorded,
    /// Same digest as before (safe replay — return cached Result).
    ReplaySame {
        /// Prior Result for this key.
        result: ResultMsg,
    },
}
