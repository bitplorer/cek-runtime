//! Domain + driver structure validation (core signing / acceptance gate).
//!
//! Ensures consistent naming across runtimes before a domain-stdlib is accepted.

/// Rules enforced at domain-stdlib acceptance / contract signing.
#[derive(Debug, Clone)]
pub struct StructureRules {
    /// Maximum dots allowed inside a domain ns (e.g. `ui.dom` = 1).
    pub max_ns_dots: usize,
    /// Forbidden domain name prefixes.
    pub forbidden_prefixes: &'static [&'static str],
    /// Reserved Baseline domain names that cannot be re-declared as domain-stdlibs.
    pub reserved_baseline_domains: &'static [&'static str],
}

impl Default for StructureRules {
    fn default() -> Self {
        Self {
            max_ns_dots: 2,
            forbidden_prefixes: &["cek.", "sys.", "_"],
            reserved_baseline_domains: &["kv", "log"],
        }
    }
}

/// Validation error for domain / driver structure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructureError {
    /// Human-readable reason.
    pub reason: String,
}

impl std::fmt::Display for StructureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "structure: {}", self.reason)
    }
}

impl std::error::Error for StructureError {}

/// Validate a domain name against structure rules.
pub fn validate_domain_name(domain: &str, rules: &StructureRules) -> Result<(), StructureError> {
    if domain.is_empty() {
        return Err(StructureError {
            reason: "domain name is empty".into(),
        });
    }
    if domain.starts_with('.') || domain.ends_with('.') {
        return Err(StructureError {
            reason: format!("domain '{domain}' has leading/trailing dot"),
        });
    }
    if domain.contains("..") {
        return Err(StructureError {
            reason: format!("domain '{domain}' contains empty segment"),
        });
    }
    let dots = domain.matches('.').count();
    if dots > rules.max_ns_dots {
        return Err(StructureError {
            reason: format!(
                "domain '{domain}' has {dots} dots; max is {}",
                rules.max_ns_dots
            ),
        });
    }
    for prefix in rules.forbidden_prefixes {
        if domain.starts_with(prefix) {
            return Err(StructureError {
                reason: format!("domain '{domain}' uses forbidden prefix '{prefix}'"),
            });
        }
    }
    for reserved in rules.reserved_baseline_domains {
        if domain == *reserved {
            return Err(StructureError {
                reason: format!("domain '{domain}' is reserved Baseline and cannot be re-declared"),
            });
        }
    }
    if !domain
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.')
    {
        return Err(StructureError {
            reason: format!(
                "domain '{domain}' has invalid characters (use lowercase, digits, '.')"
            ),
        });
    }
    Ok(())
}

/// Validate an op name (the `name` half of a pair).
pub fn validate_op_name(name: &str) -> Result<(), StructureError> {
    if name.is_empty() {
        return Err(StructureError {
            reason: "op name is empty".into(),
        });
    }
    if name.contains('.') {
        return Err(StructureError {
            reason: format!("op name '{name}' must not contain dots (dots belong in ns)"),
        });
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
    {
        return Err(StructureError {
            reason: format!("op name '{name}' has invalid characters"),
        });
    }
    Ok(())
}

/// Validate a full pair. Known seed domains skip the reserved-name check.
pub fn validate_pair(ns: &str, name: &str, rules: &StructureRules) -> Result<(), StructureError> {
    let mut rules = rules.clone();
    if matches!(ns, "kv" | "log" | "ui.dom") {
        rules.reserved_baseline_domains = &[];
    }
    validate_domain_name(ns, &rules)?;
    validate_op_name(name)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_well_formed() {
        let r = StructureRules::default();
        assert!(validate_domain_name("ui.dom", &r).is_ok());
        assert!(validate_domain_name("nav", &r).is_ok());
        assert!(validate_op_name("morph").is_ok());
        assert!(validate_pair("ui.dom", "morph", &r).is_ok());
    }

    #[test]
    fn rejects_bad_structure() {
        let r = StructureRules::default();
        assert!(validate_domain_name("", &r).is_err());
        assert!(validate_domain_name("cek.secret", &r).is_err());
        assert!(validate_domain_name("kv", &r).is_err());
        assert!(validate_op_name("dom.morph").is_err());
        assert!(validate_domain_name("UI.DOM", &r).is_err());
    }
}
