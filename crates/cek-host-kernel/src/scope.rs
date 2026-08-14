//! Cap scope attenuation — Host policy.
//!
//! Law: scopes may only **narrow**. Empty `Cap.scopes` means unrestricted.
//! A non-empty list is an allow-list over resources the Intent may touch.
//! Blank tokens are refuse (unclear authority).

use crate::{HostError, HostResult};
use cek_contract::{
    Intent, ACTION_KV_DELETE, ACTION_KV_WRITE, ACTION_LOG_APPEND, ACTION_UI_MORPH,
    ACTION_UI_RESTORE,
};

/// Resource the Intent would touch: `(kind, name)` e.g. `("kv", "greeting")`.
pub fn resource_of(intent: &Intent) -> (String, String) {
    match intent.action.as_str() {
        ACTION_KV_WRITE | ACTION_KV_DELETE => {
            let name = intent
                .args
                .get("key")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            ("kv".into(), name)
        }
        ACTION_UI_MORPH | ACTION_UI_RESTORE => {
            let name = intent
                .args
                .get("target")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            ("ui".into(), name)
        }
        ACTION_LOG_APPEND => ("log".into(), String::new()),
        other => ("action".into(), other.to_string()),
    }
}

/// True if a single scope token permits this resource.
///
/// Tokens:
/// - `kv` / `ui` / `log` — whole kind
/// - `kv:name` / `ui:name` — exact name
/// - `name` — exact name (any kind)
/// - `kv:*` — all names of that kind
///
/// Empty tokens and `kind:` (empty name) never allow.
pub fn scope_allows(scope: &str, kind: &str, name: &str) -> bool {
    let scope = scope.trim();
    if scope.is_empty() {
        return false;
    }
    if scope == kind {
        return true;
    }
    if !name.is_empty() && scope == name {
        return true;
    }
    if let Some((k, n)) = scope.split_once(':') {
        if k == kind && (n == "*" || (!n.is_empty() && n == name)) {
            return true;
        }
    }
    false
}

/// Fail closed: non-empty scopes must allow the Intent resource.
/// Blank tokens refuse (unclear).
pub fn check_scopes(intent: &Intent) -> HostResult<()> {
    let scopes = &intent.cap.scopes;
    if scopes.is_empty() {
        return Ok(());
    }
    if scopes.iter().any(|s| s.trim().is_empty()) {
        return Err(HostError::Authority(
            "empty scope token is not allowed".into(),
        ));
    }
    let (kind, name) = resource_of(intent);
    if scopes.iter().any(|s| scope_allows(s, &kind, &name)) {
        return Ok(());
    }
    Err(HostError::Authority(format!(
        "scope denied: `{kind}:{name}` not in {:?}",
        scopes
    )))
}

/// True iff `child` is a narrowing of `parent`.
///
/// - Parent empty (unrestricted) → any non-blank child (including empty list) is allowed.
/// - Parent non-empty → child must be non-empty and every child token must
///   be implied by some parent token (no widen).
/// - Blank tokens are never a valid narrowing.
pub fn can_attenuate(parent: &[String], child: &[String]) -> bool {
    if child.iter().any(|s| s.trim().is_empty()) {
        return false;
    }
    if parent.is_empty() {
        return true;
    }
    if child.is_empty() {
        return false;
    }
    child.iter().all(|c| parent.iter().any(|p| implies(p, c)))
}

/// Parent token implies child token (equal, or kind covers kind:name).
fn implies(parent: &str, child: &str) -> bool {
    let parent = parent.trim();
    let child = child.trim();
    if parent.is_empty() || child.is_empty() {
        return false;
    }
    if parent == child {
        return true;
    }
    if let Some((ck, cn)) = child.split_once(':') {
        if cn.is_empty() {
            return false;
        }
        if parent == ck {
            return true;
        }
        if let Some((pk, pn)) = parent.split_once(':') {
            return pk == ck && (pn == "*" || pn == cn);
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use cek_contract::{Cap, Intent};
    use serde_json::json;
    use std::collections::BTreeMap;

    fn intent_scoped(action: &str, key: &str, scopes: &[&str]) -> Intent {
        let mut args = BTreeMap::new();
        if action.starts_with("kv.") {
            args.insert("key".into(), json!(key));
            args.insert("value".into(), json!(1));
        } else if action.starts_with("ui.") {
            args.insert("target".into(), json!(key));
            args.insert("patch".into(), json!({"t": 1}));
        }
        Intent {
            action: action.into(),
            args,
            cap: Cap {
                id: "c".into(),
                action: action.into(),
                sealed_args_bind: None,
                not_after: None,
                once: false,
                subject: None,
                scopes: scopes.iter().map(|s| (*s).to_string()).collect(),
                sig: None,
                law_generation: None,
            },
            trace: None,
            idempotency_key: None,
            activity_id: None,
        }
    }

    #[test]
    fn empty_scopes_allow_all() {
        assert!(check_scopes(&intent_scoped("kv.write", "a", &[])).is_ok());
    }

    #[test]
    fn exact_and_kind_scopes() {
        assert!(check_scopes(&intent_scoped("kv.write", "a", &["kv:a"])).is_ok());
        assert!(check_scopes(&intent_scoped("kv.write", "a", &["kv"])).is_ok());
        assert!(check_scopes(&intent_scoped("kv.write", "a", &["kv:*"])).is_ok());
        assert!(check_scopes(&intent_scoped("kv.write", "b", &["kv:a"])).is_err());
        assert!(check_scopes(&intent_scoped("ui.morph", "hdr", &["ui:hdr"])).is_ok());
        assert!(check_scopes(&intent_scoped("ui.morph", "hdr", &["kv:hdr"])).is_err());
    }

    #[test]
    fn blank_scope_token_refuses() {
        assert!(check_scopes(&intent_scoped("kv.write", "a", &[""])).is_err());
        assert!(check_scopes(&intent_scoped("kv.write", "a", &["  "])).is_err());
        assert!(!scope_allows("log:", "log", ""));
        assert!(!scope_allows("", "kv", "a"));
    }

    #[test]
    fn attenuate_cannot_widen() {
        let parent = vec!["kv:a".into(), "kv:b".into()];
        assert!(can_attenuate(&parent, &["kv:a".into()]));
        assert!(!can_attenuate(&parent, &["kv:c".into()]));
        assert!(!can_attenuate(&parent, &[]));
        assert!(can_attenuate(&[], &["kv:a".into()]));
        assert!(can_attenuate(&["kv".into()], &["kv:a".into()]));
        assert!(!can_attenuate(&["kv:a".into()], &["kv".into()]));
        assert!(!can_attenuate(&["kv".into()], &["".into()]));
    }
}
