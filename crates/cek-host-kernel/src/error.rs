//! Host errors.

use thiserror::Error;

/// Host-side failures.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum HostError {
    /// Cap path failed (maps to authority_refusal).
    #[error("authority: {0}")]
    Authority(String),
    /// After Cap ok, handler/policy miss.
    #[error("dispatch: {0}")]
    Dispatch(String),
    /// Once-store unavailable under fail-closed policy.
    #[error("once store unavailable")]
    OnceStoreDown,
    /// Idempotency store unavailable under fail-closed policy.
    #[error("idempotency store unavailable")]
    IdemStoreDown,
    /// Lineage store failure.
    #[error("lineage: {0}")]
    Lineage(String),
}

/// Host result alias.
pub type HostResult<T> = Result<T, HostError>;
