//! # cek-host-kernel
//!
//! Host is the **decide** role: mint, verify Cap, once-consume, dispatch,
//! lineage, project Ops, reverse.
//!
//! ## Aging design
//!
//! - Pipeline stages are ordered and fail closed.
//! - [`BoundAsk`] is the only token that unlocks dispatch — Cap refuse cannot
//!   produce mutate Ops.
//! - Peer never lives inside this crate; Host does not depend on Peer.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod bound;
mod host;
mod once;
mod lineage;
mod error;

pub use bound::*;
pub use host::*;
pub use once::*;
pub use lineage::*;
pub use error::*;
