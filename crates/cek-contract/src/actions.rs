//! Intent **actions** (Host verbs) vs Ops (Peer apply data).
//!
//! | Action (Intent / Cap) | Projected Op(s) |
//! |-----------------------|-----------------|
//! | `kv.write` | `kv.set` |
//! | `kv.delete` | `kv.delete` |
//! | `log.append` | `log.append` |
//! | `ui.morph` | `ui.dom.morph` |
//! | `ui.restore` | `ui.dom.restore` |
//!
//! Actions are never applied by the Peer. Ops are never submitted as Intents.

/// `kv.write` — Host action that projects `kv.set`.
pub const ACTION_KV_WRITE: &str = "kv.write";
/// `kv.delete` — Host action that projects `kv.delete`.
pub const ACTION_KV_DELETE: &str = "kv.delete";
/// `log.append` — Host action that projects `log.append`.
pub const ACTION_LOG_APPEND: &str = "log.append";
/// `ui.morph` — Host action that projects `ui.dom.morph`.
pub const ACTION_UI_MORPH: &str = "ui.morph";
/// `ui.restore` — Host action that projects `ui.dom.restore`.
pub const ACTION_UI_RESTORE: &str = "ui.restore";

/// True if `action` is a known Host verb in this reference.
pub fn is_known_action(action: &str) -> bool {
    matches!(
        action,
        ACTION_KV_WRITE
            | ACTION_KV_DELETE
            | ACTION_LOG_APPEND
            | ACTION_UI_MORPH
            | ACTION_UI_RESTORE
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actions_are_not_ops() {
        assert_ne!(ACTION_UI_MORPH, "ui.dom.morph");
        assert_ne!(ACTION_KV_WRITE, "kv.set");
        assert!(is_known_action(ACTION_UI_MORPH));
        assert!(!is_known_action("ui.dom.morph"));
    }
}
