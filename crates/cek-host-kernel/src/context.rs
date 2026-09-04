//! Activity Context mediation — LAW §8 / CORE 07.
//!
//! Context is the mediated visible world of an Activity: not ambient authority,
//! not a Cap substitute, not a global bag of power.
//!
//! - **inject** declares required names/services. Undeclared access fails closed.
//! - **limit** restricts what may be seen or done. Only narrows (A8).
//! - **isolate** separates a slice so names/services do not leak across Activities.
//!
//! `limit` ≠ `isolate`. They must not be collapsed.
//!
//! **Not** [`crate::scope`] / `Cap.scopes` / [`Host::attenuate`] (Cap allow-list).
//! **Not** crate `07-isolation/` (process/WASM Peer split). Token grammar is
//! reused only as `kv:name` matching; the Host path is Context.

use crate::scope::{can_attenuate, resource_of, scope_allows};
use crate::{Host, HostError, HostResult};
use cek_contract::{Context, Intent};
use std::collections::BTreeMap;

#[derive(Default)]
pub(crate) struct ContextIndex {
    by_activity: BTreeMap<String, Context>,
}

fn tokens_allow(tokens: &[String], kind: &str, name: &str) -> bool {
    tokens.iter().any(|t| scope_allows(t, kind, name))
}

/// Tokens that make up an isolated slice: inject if present, else limit.
fn slice_tokens(ctx: &Context) -> &[String] {
    if !ctx.injected.is_empty() {
        &ctx.injected
    } else {
        &ctx.limits
    }
}

fn parse_token(token: &str) -> (String, String) {
    let token = token.trim();
    if let Some((k, n)) = token.split_once(':') {
        (k.to_string(), n.to_string())
    } else {
        (token.to_string(), String::new())
    }
}

fn slice_allows_token(slice: &[String], token: &str) -> bool {
    let (k, n) = parse_token(token);
    if k.is_empty() {
        return false;
    }
    if n.is_empty() || n == "*" {
        return slice.iter().any(|s| {
            let s = s.trim();
            s == k || s == format!("{k}:*") || s.starts_with(&format!("{k}:"))
        });
    }
    tokens_allow(slice, &k, &n)
}

fn slices_overlap(a: &[String], b: &[String]) -> bool {
    a.iter().any(|t| slice_allows_token(b, t)) || b.iter().any(|t| slice_allows_token(a, t))
}

fn require_activity_id(activity_id: &str) -> HostResult<&str> {
    if activity_id.trim().is_empty() {
        return Err(HostError::Authority(
            "empty activity_id is not allowed".into(),
        ));
    }
    Ok(activity_id)
}

fn require_names(names: &[String]) -> HostResult<()> {
    if names.is_empty() {
        return Err(HostError::Authority(
            "empty Context name list is not allowed".into(),
        ));
    }
    if names.iter().any(|s| s.trim().is_empty()) {
        return Err(HostError::Authority(
            "empty Context name token is not allowed".into(),
        ));
    }
    Ok(())
}

impl Host {
    /// Declare what an Activity requires from its Context (`inject`, LAW §8).
    ///
    /// Undeclared access on later [`Host::submit`] fails closed. Additive.
    /// Does not grant a parent's missing rights. Names isolated by another
    /// Activity are refused (isolate holds).
    pub fn inject(&self, activity_id: &str, names: Vec<String>) -> HostResult<Context> {
        let activity_id = require_activity_id(activity_id)?;
        require_names(&names)?;
        let mut g = self
            .contexts
            .lock()
            .map_err(|_| HostError::Authority("context lock".into()))?;
        for (other_id, other) in &g.by_activity {
            if other_id == activity_id || !other.isolated {
                continue;
            }
            let slice = slice_tokens(other);
            if slices_overlap(&names, slice) {
                return Err(HostError::Authority(format!(
                    "isolate holds: names overlap Activity `{other_id}` slice"
                )));
            }
        }
        let ctx = g
            .by_activity
            .entry(activity_id.to_string())
            .or_insert_with(|| Context {
                activity_id: activity_id.to_string(),
                injected: Vec::new(),
                limits: Vec::new(),
                isolated: false,
            });
        for n in names {
            if !ctx.injected.iter().any(|e| e == &n) {
                ctx.injected.push(n);
            }
        }
        Ok(ctx.clone())
    }

    /// Restrict what an Activity may see or do (`limit`, LAW §8).
    ///
    /// Only **narrows** (A8). Does not grant missing rights. Distinct from
    /// [`Host::isolate`] — limit does not by itself prevent leak to other Activities.
    pub fn limit(&self, activity_id: &str, names: Vec<String>) -> HostResult<Context> {
        let activity_id = require_activity_id(activity_id)?;
        require_names(&names)?;
        let mut g = self
            .contexts
            .lock()
            .map_err(|_| HostError::Authority("context lock".into()))?;
        let ctx = g
            .by_activity
            .entry(activity_id.to_string())
            .or_insert_with(|| Context {
                activity_id: activity_id.to_string(),
                injected: Vec::new(),
                limits: Vec::new(),
                isolated: false,
            });
        let parent = if !ctx.limits.is_empty() {
            ctx.limits.clone()
        } else {
            ctx.injected.clone()
        };
        if !can_attenuate(&parent, &names) {
            return Err(HostError::Authority(
                "limit would widen Context (LAW §8 / A8)".into(),
            ));
        }
        ctx.limits = names;
        Ok(ctx.clone())
    }

    /// Separate this Activity's Context slice (`isolate`, LAW §8).
    ///
    /// Names/services in the slice do not leak across Activities. Distinct from
    /// [`Host::limit`]. Only narrows (A8) — cannot undo.
    pub fn isolate(&self, activity_id: &str) -> HostResult<Context> {
        let activity_id = require_activity_id(activity_id)?;
        let mut g = self
            .contexts
            .lock()
            .map_err(|_| HostError::Authority("context lock".into()))?;
        let slice = g
            .by_activity
            .get(activity_id)
            .map(slice_tokens)
            .unwrap_or(&[])
            .to_vec();
        for (other_id, other) in &g.by_activity {
            if other_id == activity_id || !other.isolated {
                continue;
            }
            if slices_overlap(&slice, slice_tokens(other)) {
                return Err(HostError::Authority(format!(
                    "isolate holds: names overlap Activity `{other_id}` slice"
                )));
            }
        }
        let ctx = g
            .by_activity
            .entry(activity_id.to_string())
            .or_insert_with(|| Context {
                activity_id: activity_id.to_string(),
                injected: Vec::new(),
                limits: Vec::new(),
                isolated: false,
            });
        ctx.isolated = true;
        Ok(ctx.clone())
    }

    /// Current Context for an Activity, if one was injected / limited / isolated.
    pub fn context_of(&self, activity_id: &str) -> Option<Context> {
        self.contexts
            .lock()
            .ok()?
            .by_activity
            .get(activity_id)
            .cloned()
    }

    pub(crate) fn drop_context(&self, activity_id: &str) {
        if let Ok(mut g) = self.contexts.lock() {
            g.by_activity.remove(activity_id);
        }
    }

    /// Mediate Intent against Activity Context. Fail closed (empty Ops via caller).
    pub(crate) fn mediate_context(&self, intent: &Intent) -> HostResult<()> {
        let (kind, name) = resource_of(intent);
        let aid = intent.activity_id.as_deref().filter(|s| !s.is_empty());
        let g = self
            .contexts
            .lock()
            .map_err(|_| HostError::Authority("context lock".into()))?;

        for (other_id, ctx) in &g.by_activity {
            if !ctx.isolated {
                continue;
            }
            if aid == Some(other_id.as_str()) {
                continue;
            }
            let slice = slice_tokens(ctx);
            if !slice.is_empty() && tokens_allow(slice, &kind, &name) {
                return Err(HostError::Authority(format!(
                    "isolate holds: `{kind}:{name}` is in Activity `{other_id}` slice"
                )));
            }
        }

        if let Some(aid) = aid {
            if let Some(ctx) = g.by_activity.get(aid) {
                if !ctx.injected.is_empty() && !tokens_allow(&ctx.injected, &kind, &name) {
                    return Err(HostError::Authority(format!(
                        "undeclared inject: `{kind}:{name}` not in {:?}",
                        ctx.injected
                    )));
                }
                if !ctx.limits.is_empty() && !tokens_allow(&ctx.limits, &kind, &name) {
                    return Err(HostError::Authority(format!(
                        "over-limit: `{kind}:{name}` not in {:?}",
                        ctx.limits
                    )));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::Host;

    #[test]
    fn empty_activity_id_refuses_context_apis() {
        let host = Host::with_clock(1000);
        assert!(host.inject("", vec!["kv:a".into()]).is_err());
        assert!(host.limit("  ", vec!["kv:a".into()]).is_err());
        assert!(host.isolate("").is_err());
    }

    #[test]
    fn blank_or_empty_names_refuse() {
        let host = Host::with_clock(1000);
        assert!(host.inject("act", vec![]).is_err());
        assert!(host.inject("act", vec!["".into()]).is_err());
        assert!(host.limit("act", vec!["  ".into()]).is_err());
    }

    #[test]
    fn limit_cannot_widen_injected() {
        let host = Host::with_clock(1000);
        host.inject("act", vec!["kv:greeting".into()]).unwrap();
        assert!(host.limit("act", vec!["kv".into()]).is_err());
        assert!(host.limit("act", vec!["kv:other".into()]).is_err());
        let ctx = host.limit("act", vec!["kv:greeting".into()]).unwrap();
        assert_eq!(ctx.limits, vec!["kv:greeting".to_string()]);
    }

    #[test]
    fn isolate_overlaps_other_slice() {
        let host = Host::with_clock(1000);
        host.inject("a", vec!["kv:secret".into()]).unwrap();
        host.isolate("a").unwrap();
        assert!(host.inject("b", vec!["kv:secret".into()]).is_err());
        host.inject("b", vec!["kv:other".into()]).unwrap();
        assert!(host.isolate("b").is_ok());
    }

    #[test]
    fn limit_is_not_isolate() {
        let host = Host::with_clock(1000);
        host.inject("a", vec!["kv:secret".into()]).unwrap();
        host.limit("a", vec!["kv:secret".into()]).unwrap();
        let ctx = host.context_of("a").unwrap();
        assert!(!ctx.isolated);
        assert!(!ctx.limits.is_empty());
    }
}
