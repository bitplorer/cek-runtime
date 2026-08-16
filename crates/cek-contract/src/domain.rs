//! Isolated Domain catalog — sole source of Domain FQs.
//!
//! Pack shape: `family.scope` (at least one dot). FQ: `pack.op`.
//! Wire identity is the **pair** `(ns, name)`, not the concatenated string.
//! `name` is a single token (no dots). `ns` for a Domain Op is the pack.
//! Baseline is **not** in this module (`baseline.rs`).
//! Via negativa: undeclared pair is illegal.

use crate::baseline;

/// One Domain Op declaration (frozen after crate compile).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DomainOpDecl {
    /// Pack id (`family.scope`). Equals `ns` on the wire.
    pub pack: &'static str,
    /// Wire `ns` (the pack).
    pub ns: &'static str,
    /// Wire `name` — single token, last segment of `fq`.
    pub name: &'static str,
    /// Fully-qualified Op name (`ns.name`). Display / apply-set only.
    pub fq: &'static str,
    /// `inverse` | `snapshot` | `NonReversible`
    pub reverse: &'static str,
    /// `lower` | `skip` | `NonReversible`
    pub lowering: &'static str,
}

/// Declared Domain Ops. Grow only with a complete decl + driver.
pub const DOMAIN_DECLS: &[DomainOpDecl] = &[
    DomainOpDecl {
        pack: "ui.dom",
        ns: "ui.dom",
        name: "morph",
        fq: "ui.dom.morph",
        reverse: "snapshot",
        lowering: "lower",
    },
    DomainOpDecl {
        pack: "ui.dom",
        ns: "ui.dom",
        name: "restore",
        fq: "ui.dom.restore",
        reverse: "NonReversible",
        lowering: "lower",
    },
];

/// Domain FQ list (startup-frozen). Derived from decls.
pub const DOMAIN_FQS: &[&str] = &["ui.dom.morph", "ui.dom.restore"];

/// True if `name` is a single legal token (no dots).
pub fn name_is_token(name: &str) -> bool {
    token_ok(name)
}

/// True if `(ns, name)` is a declared Domain pair.
pub fn is_domain_pair(ns: &str, name: &str) -> bool {
    name_is_token(name) && DOMAIN_DECLS.iter().any(|d| d.ns == ns && d.name == name)
}

/// True if concatenated `fq` matches a declared Domain FQ.
///
/// Prefer [`is_domain_pair`]. This exists for apply-set membership of already-legal pairs.
pub fn is_domain_fq(fq: &str) -> bool {
    DOMAIN_FQS.contains(&fq)
}

/// Pack that owns the declared pair `(ns, name)`.
pub fn pack_of_pair(ns: &str, name: &str) -> Option<&'static str> {
    DOMAIN_DECLS
        .iter()
        .find(|d| d.ns == ns && d.name == name)
        .map(|d| d.pack)
}

/// Pack that owns an exact declared FQ string (not a split-alias).
pub fn pack_of(fq: &str) -> Option<&'static str> {
    DOMAIN_DECLS.iter().find(|d| d.fq == fq).map(|d| d.pack)
}

/// Baseline ∪ declared Domain, **pair identity**.
///
/// `("ui.dom", "morph")` is legal. `("ui", "dom.morph")` is not —
/// concatenation is not identity.
pub fn is_legal_fq(ns: &str, name: &str) -> bool {
    is_legal_pair(ns, name)
}

/// Same as [`is_legal_fq`] — the name that states the rule.
pub fn is_legal_pair(ns: &str, name: &str) -> bool {
    if !name_is_token(name) {
        return false;
    }
    baseline::is_baseline(ns, name) || is_domain_pair(ns, name)
}

/// Domain pack must be `family.scope` (one or more dots, lowercase tokens).
pub fn pack_is_scoped(pack: &str) -> bool {
    let mut it = pack.split('.');
    match (it.next(), it.next()) {
        (Some(a), Some(b)) => token_ok(a) && token_ok(b) && it.all(token_ok),
        _ => false,
    }
}

fn token_ok(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
}

/// Project: keep Baseline + Domain pairs whose pack is in `allowed_packs`.
/// Unknown **pairs** (including split-aliases): skip (`tolerant`) or drop the batch (`strict`).
pub fn project_domain_ops<'a>(
    ops: impl IntoIterator<Item = &'a crate::Op>,
    allowed_packs: &[&str],
    unknown: &str,
) -> Vec<crate::Op> {
    let mut out = Vec::new();
    for op in ops {
        if baseline::is_baseline(&op.ns, &op.name) {
            out.push(op.clone());
            continue;
        }
        match pack_of_pair(&op.ns, &op.name) {
            Some(pack) if allowed_packs.iter().any(|p| *p == pack) => out.push(op.clone()),
            Some(_) => {
                // declared but not in profile → skip (or lower later)
            }
            None if unknown == "strict" => return Vec::new(),
            None => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{baseline, ui_morph, Op};
    use serde_json::json;

    #[test]
    fn decls_match_fq_list_and_scope() {
        assert_eq!(DOMAIN_DECLS.len(), DOMAIN_FQS.len());
        for d in DOMAIN_DECLS {
            assert!(pack_is_scoped(d.pack), "{}", d.pack);
            assert_eq!(d.ns, d.pack);
            assert_eq!(d.fq, format!("{}.{}", d.ns, d.name));
            assert!(name_is_token(d.name), "{}", d.name);
            assert!(d.fq.starts_with(d.pack), "{}", d.fq);
            assert!(DOMAIN_FQS.contains(&d.fq));
            assert!(is_legal_pair(d.ns, d.name));
        }
        assert!(is_legal_fq("kv", "set"));
        assert!(is_legal_fq("ui.dom", "morph"));
        assert!(!is_legal_fq("ui", "focus"));
        assert!(!is_legal_fq("nav", "push"));
        assert!(!is_legal_fq("signal", "set"));
        assert!(!is_legal_fq("kv", "merge"));
        assert!(!baseline::is_baseline("ui.dom", "morph"));
        assert_eq!(pack_of("ui.dom.morph"), Some("ui.dom"));
        assert_eq!(pack_of_pair("ui.dom", "morph"), Some("ui.dom"));
        assert!(pack_of("ui.focus").is_none());
        assert!(pack_of_pair("ui", "focus").is_none());
    }

    #[test]
    fn split_alias_is_illegal() {
        // Concatenation is not identity. These all hash to the same FQ string
        // as a legal Op, and must still be rejected.
        assert!(!is_legal_pair("ui", "dom.morph"));
        assert!(!is_legal_pair("ui.dom.morph", ""));
        assert!(!is_legal_pair("", "ui.dom.morph"));
        assert!(!name_is_token("dom.morph"));
        assert!(pack_of_pair("ui", "dom.morph").is_none());
        assert!(!is_domain_pair("ui", "dom.morph"));
        assert!(is_domain_fq("ui.dom.morph")); // string membership ≠ pair
    }

    #[test]
    fn project_skips_undeclared_tolerant() {
        let ops = vec![
            baseline::kv_set("a", json!(1)),
            Op {
                ns: "nav".into(),
                name: "push".into(),
                payload: json!({"path": "/x"}),
            },
        ];
        let out = project_domain_ops(&ops, &["ui.dom"], "tolerant");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].fq(), "kv.set");
    }

    #[test]
    fn project_strict_unknown_empties_batch() {
        let ops = vec![
            baseline::kv_set("a", json!(1)),
            Op {
                ns: "ui".into(),
                name: "toast".into(),
                payload: json!({"message": "x"}),
            },
        ];
        let out = project_domain_ops(&ops, &["ui.dom"], "strict");
        assert!(out.is_empty());
    }

    #[test]
    fn project_strict_rejects_split_alias() {
        let ops = vec![
            baseline::kv_set("a", json!(1)),
            Op {
                ns: "ui".into(),
                name: "dom.morph".into(),
                payload: json!({"target": "hdr", "patch": {"t": 1}}),
            },
        ];
        let out = project_domain_ops(&ops, &["ui.dom"], "strict");
        assert!(out.is_empty());
        let out = project_domain_ops(&ops, &["ui.dom"], "tolerant");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].ns, "kv");
    }

    #[test]
    fn ui_morph_still_legal() {
        let m = ui_morph("hdr", json!({"t": 1}), None);
        assert!(is_legal_pair(&m.ns, &m.name));
        assert_eq!((m.ns.as_str(), m.name.as_str()), ("ui.dom", "morph"));
    }
}
