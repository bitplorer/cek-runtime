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
//! - **Actions** (`kv.write`, `ui.morph`) are Host verbs. **Ops** (`kv.set`, `ui.dom.morph`) are Peer data.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod actions;
pub mod baseline;
pub mod digest;
pub mod domain;
mod error;
pub mod pair;
pub mod structure;
pub mod types;
pub mod ui;
mod vectors;

pub use actions::*;
pub use baseline::*;
pub use digest::*;
pub use domain::{
    is_domain_fq, is_domain_pair, is_legal_fq, is_legal_pair, name_is_token, pack_is_scoped,
    pack_of, pack_of_pair, project_domain_ops, DomainOpDecl, DOMAIN_DECLS, DOMAIN_FQS,
};
pub use error::*;
pub use pair::{baseline_ui_pairset, Pair, PairSet};
pub use structure::{
    validate_domain_name, validate_op_name, validate_pair, StructureError, StructureRules,
};
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

/// UI domain profile name — Baseline + `ui.dom.*`.
pub const PROFILE_UI: &str = "ui";

#[cfg(test)]
mod digest_props;
#[cfg(test)]
mod types_props;
#[cfg(test)]
mod vector_tests;
