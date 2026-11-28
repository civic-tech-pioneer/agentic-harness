//! The calls of 2.3 that work by profile name: `check(profile, overlay)` and `profiles()` (9.1).
//! Both register eligible callers in `clients` and read profile state; neither starts anything.

use crate::access;
use crate::check::check;
use crate::error::Refusal;
use crate::profile::{parse_profile, Backend};
use crate::sessions::Kernel;
use crate::state::State;
use crate::Host;

/// The most profiles `profiles` lists (the rest are left out).
pub const MAX_PROFILES: usize = 64;

/// One profile a client may use, as `profiles` lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileInfo {
    pub name: String,
    pub backend: String,
    pub model: Option<String>,
}

fn one(field: &str, reason: &str) -> Vec<Refusal> {
    vec![Refusal {
        field: field.to_string(),
        backend: String::new(),
        model: String::new(),
        reason: reason.to_string(),
    }]
}

impl<S: State, H: Host> Kernel<S, H> {
    /// 2.3 `check`: the refusals `start` would give for this profile and overlay, none started.
    pub fn check_named(&mut self, caller: &str, profile: &str, overlay: &str) -> Vec<Refusal> {
        self.register_client(caller);
        if !self.may_use(caller, profile) {
            return match self.profile_access_error(caller, profile) {
                crate::error::KernelError::Refused(r) => r,
                _ => unreachable!(),
            };
        }
        let Some(json) = self.profile_json(profile) else {
            return one("profile", "no such profile");
        };
        if self
            .access_digest(caller, profile)
            .as_deref()
            .is_some_and(|d| access::digest(json.as_bytes()) != d)
        {
            return one("profile", "changed since the participant allowed it");
        }
        let parsed = parse_profile(&json).0;
        let backend = parsed
            .backend
            .as_deref()
            .and_then(|b| b.parse::<Backend>().ok())
            .unwrap_or(Backend::Claude);
        let mut out = Vec::new();
        if backend == Backend::Openai && parsed.model.is_none() {
            out.push(Refusal {
                field: "model".into(),
                backend: backend.as_str().into(),
                model: String::new(),
                reason: "an OpenAI profile needs a model".into(),
            });
        }
        let model = parsed.model.unwrap_or_else(|| "default".to_string());
        out.extend(check(&json, overlay, backend, &model, None));
        out
    }

    /// 2.3 `profiles`: the profiles `caller` may use (9.1), in the access list's order, at most
    /// 64; reads `access/<caller>` and at most 64 `profile/` keys.
    pub fn profiles(&mut self, caller: &str) -> Vec<ProfileInfo> {
        self.register_client(caller);
        let names: Vec<String> = match access::entries(&mut self.state, caller) {
            Some(entries) => {
                let index = access::index(&mut self.state);
                entries
                    .into_iter()
                    .filter(|e| access::holds(e, &index))
                    .map(|e| e.profile)
                    .collect()
            }
            None if self.may_use(caller, "scratch") => vec!["scratch".to_string()],
            None => Vec::new(),
        };
        let mut out = Vec::new();
        for name in names.into_iter().take(MAX_PROFILES) {
            let Some(json) = self.profile_json(&name) else {
                continue;
            };
            let p = parse_profile(&json).0;
            out.push(ProfileInfo {
                name,
                backend: p.backend.unwrap_or_else(|| "claude".to_string()),
                model: p.model,
            });
        }
        out
    }

    pub(crate) fn register_client(&mut self, caller: &str) {
        if caller == "page" || caller.starts_with("agent:") || caller.len() > 128 {
            return;
        }
        // An unreadable client registry grants nothing. Replacing it with a fresh list is safe:
        // clients is a display/discovery list, never a source of profile authority.
        let mut clients: Vec<String> = self
            .state
            .get("clients")
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        if clients.len() >= 64 || clients.iter().any(|c| c == caller) {
            return;
        }
        clients.push(caller.to_string());
        let _ = self
            .state
            .put("clients", serde_json::to_vec(&clients).unwrap());
    }
}
