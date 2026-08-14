//! Contract-level errors (parsing / vector load).

use thiserror::Error;

/// Errors from loading or validating contract artifacts.
#[derive(Debug, Error)]
pub enum ContractError {
    /// JSON parse failed.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    /// Vector case failed structural checks.
    #[error("vector invalid: {0}")]
    VectorInvalid(String),
    /// I/O while reading fixtures.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

/// Result alias for contract operations.
pub type ContractResult<T> = Result<T, ContractError>;
