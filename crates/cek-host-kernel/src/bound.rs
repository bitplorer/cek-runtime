//! BoundAsk — post-verify token.

use cek_contract::Intent;

/// Proof that Cap verify + once/idempotency succeeded.
///
/// Construction is private to this crate; only [`crate::Host`] can mint it
/// after the authority path succeeds.
#[derive(Debug, Clone)]
pub struct BoundAsk {
    pub(crate) intent: Intent,
    pub(crate) now: u64,
}

impl BoundAsk {
    /// Borrow the verified Intent.
    pub fn intent(&self) -> &Intent {
        &self.intent
    }

    /// Host clock used at bind time.
    pub fn now(&self) -> u64 {
        self.now
    }
}
