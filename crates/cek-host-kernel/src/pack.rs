//! Domain extension packs. Not law. Packs must not mint Caps or build BoundAsk.

use cek_contract::{Intent, Op};

/// Host-side domain pack: project an Action the kernel does not know.
///
/// Return `None` if this pack does not own the Action / Op.
pub trait DomainPack: Send + Sync {
    /// Stable pack id (`ui`, …).
    fn name(&self) -> &'static str;

    /// Project Intent → Ops. `None` = not this pack.
    fn project(&self, intent: &Intent) -> Option<Result<Vec<Op>, String>>;

    /// Inverse of a landed Op. `None` = not this pack (or non-reversible).
    fn inverse(&self, op: &Op) -> Option<Op>;
}
