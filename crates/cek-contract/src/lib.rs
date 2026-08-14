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
//! - Digests use `cek1:` SHA-256 over canonical JSON; algorithm id is part of the string.
//! - Peer cannot mint; that is enforced by Host/Peer crates + CI, not by types alone.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod baseline;
pub mod digest;
mod error;
pub mod types;
pub mod ui;
mod vectors;

pub use baseline::*;
pub use digest::*;
pub use error::*;
pub use types::*;
pub use ui::*;
pub use vectors::*;

/// Law generation this contract claims to speak.
pub const LAW_GENERATION: &str = "cek-law-1";

/// Contract crate semver (independent of law generation).
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Production profile name — receipts expected for landed-first reverse.
pub const PROFILE_PRODUCTION_V1: &str = "production-v1";

/// Baseline profile name — classic Ops only; receipts optional.
pub const PROFILE_BASELINE: &str = "baseline";

#[cfg(test)]
mod digest_props;
#[cfg(test)]
mod types_props;
#[cfg(test)]
mod vector_tests;
