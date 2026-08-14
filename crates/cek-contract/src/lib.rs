//! # cek-contract
//!
//! The **only interop product** for CEK Host and Peer kernels.
//!
//! Law meanings live in [cek-framework](https://github.com/bitplorer/cek-framework).
//! This crate freezes **shapes** used on the wire and in tests:
//! Intent, Cap, Result, Op, lineage, receipt, profile, manifest.
//!
//! ## Aging rules
//!
//! - Additive optional fields only; never rename frozen conceptual fields.
//! - Ops remain **data** (`ns`, `name`, `payload`) — no eval.
//! - `authority_refusal` must never carry mutate Ops.
//! - Peer cannot mint; that is enforced by Host/Peer crates + CI, not by types alone.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod types;
pub mod baseline;
mod vectors;
mod error;

pub use types::*;
pub use baseline::*;
pub use vectors::*;
pub use error::*;

/// Law generation this contract claims to speak.
pub const LAW_GENERATION: &str = "cek-law-1";

/// Contract crate semver (independent of law generation).
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
