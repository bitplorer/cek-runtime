//! # cek-host-kernel
//!
//! Host is the **decide** role: mint, verify Cap, once (ensure/commit), sealed-args,
//! idempotency, dispatch, lineage, project Ops, reverse.
//!
//! ## Pipeline (fail closed, ordered)
//!
//! ```text
//! verify Cap (action, expiry, sealed-args)
//!   → idempotency lookup (before once)
//!   → once ensure_available (before effects; do not burn yet)
//!   → BoundAsk
//!   → dispatch (authorized Ops)
//!   → once commit (only after successful dispatch; no burn on miss)
//!   → lineage commit (if Activity; persist optional Intent.trace)  // LAW §4: before project
//!   → project Ops onto Result          // LAW §11: ability ∪ Baseline
//!   → Result + digest
//! ```
//!
//! ## Aging design
//!
//! - [`BoundAsk`] is the only token that unlocks dispatch.
//! - Peer never lives inside this crate.
//! - Reverse prefers **landed** Ops when a receipt was annotated.
//! - [`Host::revoke`] marks the Cap revoked (LAW §5) and reverses Cap-scoped lineage (LAW §9).
//! - [`Host::mint_recovery`] mints a Recovery Cap (LAW §13); Compensation submits ordinary Intents.
//! - [`Host::inject`] / [`Host::limit`] / [`Host::isolate`] mediate Activity Context (LAW §8).
//!   `limit` ≠ `isolate`. Both only narrow (A8).
//! - Durable state is behind [`OnceBackend`] / [`IdemBackend`] / [`LineageBackend`].
//! - [`Host::for_trace`] groups related Intents (LAW §10). Trace is never Cap,
//!   never undo, never a resume ticket.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod bound;
mod context;
mod durable;
mod error;
mod host;
mod idem;
mod lineage;
mod once;
mod project;
mod scope;
mod sign;
mod store;
mod verify;

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
