//! Cap verify — action, expiry, sealed-args, subject, generation, signature.
//! No once. No project.

use crate::{Host, HostError, HostResult};
use cek_contract::{sealed_args_digest, Cap, Intent};

impl Host {
    pub(crate) fn requires_cap_sig(&self) -> bool {
        self.signing_key.is_some() || self.ed_sign.is_some() || !self.ed_trust.is_empty()
    }

    /// Cap integrity (action, expiry, sealed-args, id, scopes). No once, no idem.
    pub(crate) fn verify_cap(&self, intent: &Intent, now: u64) -> HostResult<()> {
        let cap = &intent.cap;
        if intent.action != cap.action {
            return Err(HostError::Authority(format!(
                "action mismatch: intent `{}` vs Cap `{}`",
                intent.action, cap.action
            )));
        }
        if let Some(na) = cap.not_after {
            if now >= na {
                return Err(HostError::Authority(format!(
                    "Cap expired: now={now} not_after={na}"
                )));
            }
        }
        if self.enforce_sealed {
            if let Some(ref bind) = cap.sealed_args_bind {
                let got = sealed_args_digest(&intent.args);
                if &got != bind {
                    return Err(HostError::Authority(format!(
                        "sealed-args bind mismatch: cap expects {bind}, got {got}"
                    )));
                }
            }
        }
        if intent.action.is_empty() || cap.action.is_empty() {
            return Err(HostError::Authority("empty action is not allowed".into()));
        }
        if cap.id.is_empty() {
            return Err(HostError::Authority("empty Cap id is not allowed".into()));
        }
        crate::scope::check_scopes(intent)?;
        Self::check_subject(intent)?;
        self.check_generation(&intent.cap)?;
        self.verify_sig(&intent.cap)?;
        Ok(())
    }

    /// Unset generation = legacy current. Set generation must be in the window.
    pub(crate) fn check_generation(&self, cap: &Cap) -> HostResult<()> {
        match cap.law_generation.as_deref() {
            None => Ok(()),
            Some(g) if g.trim().is_empty() => Err(HostError::Authority(
                "empty law generation is not allowed".into(),
            )),
            Some(g) if self.accepted_generations.iter().any(|a| a == g) => Ok(()),
            Some(g) => Err(HostError::Authority(format!(
                "law generation `{g}` not in {:?}",
                self.accepted_generations
            ))),
        }
    }

    /// Cap.subject set → Intent.args.subject must match. Blank bind refuses.
    pub(crate) fn check_subject(intent: &Intent) -> HostResult<()> {
        match intent.cap.subject.as_deref() {
            None => Ok(()),
            Some(s) if s.trim().is_empty() => Err(HostError::Authority(
                "empty Cap subject is not allowed".into(),
            )),
            Some(want) => {
                let got = intent.args.get("subject").and_then(|v| v.as_str());
                if got == Some(want) {
                    Ok(())
                } else {
                    Err(HostError::Authority(format!(
                        "subject bind mismatch: cap `{want}` vs presenter {got:?}"
                    )))
                }
            }
        }
    }

    pub(crate) fn verify_sig(&self, cap: &Cap) -> HostResult<()> {
        if !self.requires_cap_sig() {
            return Ok(());
        }
        let Some(sig) = cap.sig.as_deref() else {
            return Err(HostError::Authority("Cap signature required".into()));
        };
        if sig.starts_with("ed25519:") {
            if crate::sign::ed25519_valid(&self.ed_trust, cap) {
                return Ok(());
            }
            return Err(HostError::Authority("Cap signature invalid".into()));
        }
        if let Some(key) = &self.signing_key {
            if cek_contract::cap_signature_valid(key, cap) {
                return Ok(());
            }
        }
        Err(HostError::Authority("Cap signature invalid".into()))
    }
}
