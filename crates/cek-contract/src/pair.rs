//! Pair identity — (ns, name) is the legality key.
//!
//! Wire FQ strings (`ns.name`) are serialization only.
//! Concatenation must never be used as the identity key.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::{baseline, domain};

/// A single catalog pair. Identity is the two fields, not their concatenation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Pair {
    /// Namespace (may contain dots, e.g. `ui.dom`).
    pub ns: String,
    /// Op name within the namespace (e.g. `morph`).
    pub name: String,
}

impl Pair {
    /// Construct a pair.
    pub fn new(ns: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            ns: ns.into(),
            name: name.into(),
        }
    }

    /// Serialization form `ns.name` — display / wire only, never legality key.
    pub fn fq(&self) -> String {
        format!("{}.{}", self.ns, self.name)
    }

    /// Last-dot split of a display FQ. Prefer constructing from (ns, name).
    pub fn from_fq(fq: &str) -> Option<Self> {
        let idx = fq.rfind('.')?;
        if idx == 0 || idx + 1 >= fq.len() {
            return None;
        }
        Some(Self {
            ns: fq[..idx].to_string(),
            name: fq[idx + 1..].to_string(),
        })
    }
}

impl From<(&str, &str)> for Pair {
    fn from(value: (&str, &str)) -> Self {
        Self::new(value.0, value.1)
    }
}

/// Closed set of pairs — the session stamp / apply_set authority.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairSet {
    pairs: BTreeSet<Pair>,
}

impl PairSet {
    /// Empty set.
    pub fn new() -> Self {
        Self {
            pairs: BTreeSet::new(),
        }
    }

    /// Build from explicit pairs.
    pub fn from_pairs<I, P>(iter: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: Into<Pair>,
    {
        let mut s = Self::new();
        for p in iter {
            s.insert(p.into());
        }
        s
    }

    /// Insert a pair. Returns true if newly inserted.
    pub fn insert(&mut self, pair: Pair) -> bool {
        self.pairs.insert(pair)
    }

    /// True if the exact pair is present. Pair identity — not FQ string match.
    pub fn contains(&self, ns: &str, name: &str) -> bool {
        self.pairs.contains(&Pair {
            ns: ns.to_string(),
            name: name.to_string(),
        })
    }

    /// Number of pairs.
    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    /// True if empty.
    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }

    /// Iterate pairs in sorted order.
    pub fn iter(&self) -> impl Iterator<Item = &Pair> {
        self.pairs.iter()
    }

    /// Union with another set.
    pub fn union(&self, other: &PairSet) -> PairSet {
        let mut out = self.clone();
        for p in other.iter() {
            out.insert(p.clone());
        }
        out
    }

    /// Serialize as sorted FQ strings (wire / Profile.apply_set compatibility).
    pub fn to_fq_list(&self) -> Vec<String> {
        self.pairs.iter().map(|p| p.fq()).collect()
    }
}

/// Baseline ∪ UI seed as a PairSet (Phase 1 default stamp source).
pub fn baseline_ui_pairset() -> PairSet {
    let mut s = PairSet::from_pairs(baseline::BASELINE_PAIRS.iter().copied());
    for d in domain::DOMAIN_DECLS {
        s.insert(Pair::new(d.ns, d.name));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pair_identity_not_concatenation() {
        let set = PairSet::from_pairs([("ui.dom", "morph")]);
        assert!(set.contains("ui.dom", "morph"));
        assert!(!set.contains("ui", "dom.morph"));
        assert!(!set.contains("ui.dom.morph", ""));
    }

    #[test]
    fn baseline_ui_pairset_size() {
        let s = baseline_ui_pairset();
        assert_eq!(s.len(), 5);
        assert!(s.contains("kv", "set"));
        assert!(s.contains("ui.dom", "restore"));
        assert!(!s.contains("nav", "push"));
    }
}
