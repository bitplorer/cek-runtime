//! # cek-host-kernel
//!
//! Host is the **decide** role: mint, verify Cap, once-consume, sealed-args,
//! idempotency, dispatch, lineage, project Ops, reverse.
//!
//! ## Pipeline (fail closed, ordered)
//!
//! ```text
//! verify Cap (action, expiry, sealed-args)
//!   → idempotency lookup (before once)
//!   → once ensure_available (before effects; do not burn yet)
//!   → BoundAsk
//!   → project Ops
//!   → once commit (only after successful project)
//!   → lineage commit (if Activity)
//!   → Result + digest
//! ```
//!
//! ## Aging design
//!
//! - [`BoundAsk`] is the only token that unlocks dispatch.
//! - Peer never lives inside this crate.
//! - Reverse prefers **landed** Ops when a receipt was annotated.
//! - Durable state is behind [`OnceBackend`] / [`IdemBackend`] / [`LineageBackend`].

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod bound;
mod durable;
mod error;
mod host;
mod idem;
mod lineage;
mod once;
mod scope;
mod sign;
mod store;

pub use bound::*;
pub use durable::*;
pub use error::*;
pub use host::*;
pub use idem::*;
pub use lineage::*;
pub use once::*;
pub use scope::{can_attenuate, check_scopes, resource_of, scope_allows};
pub use sign::{parse_hex32, public_key as ed25519_public_from_seed};
pub use store::*;

#[cfg(test)]
mod batteries;
#[cfg(test)]
mod fail_closed;
#[cfg(test)]
mod props;
