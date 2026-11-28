//! Participant-controlled profile manager (TECHNICAL 9.10).
//! The host's answered access stream, never the requesting page, is the write authority.

use crate::access::{self, AccessEntry, OfferLink, ProfileIndexEntry};
use crate::error::{KernelError, Refusal};
use crate::profile::{grammar_problems, parse_profile};
use crate::sessions::{Host, Kernel};
use crate::state::State;
use serde::{
    de::{MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashSet},
    fmt,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessError {
    NotPage,
    NoMod,
    TooLarge,
    TooMany,
}
impl AccessError {
    pub fn code(self) -> &'static str {
        match self {
            Self::NotPage => "not-page",
            Self::NoMod => "no-mod",
            Self::TooLarge => "too-large",
            Self::TooMany => "too-many",
        }
    }
}

/// Native fake of the exact four TECHNICAL 15 question variants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccessQuestion {
    Manage,
    Allow {
        client: String,
        profile: String,
        reach: String,
    },
    Store {
        profile: String,
        text: String,
    },
    Remove {
        profile: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfferedProfile {
    pub name: String,
    pub description: String,
    pub profile: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Standing {
    Current,
    Changed,
    Stale,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredProfile {
    pub name: String,
    pub digest: String,
    pub rev: u32,
    pub bytes: u32,
    pub offer: Option<String>,
    pub stale: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Allowed {
    pub profile: String,
    pub digest: String,
    pub standing: Standing,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientInfo {
    pub identity: String,
    pub allowed: Vec<Allowed>,
    pub asking: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct BoundStream {
    stream: String,
    connection: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(crate) enum Pending {
    Allow {
        client: String,
        profile: String,
        digest: String,
        rev: u32,
    },
    Store {
        profile: String,
        digest: String,
        base: Option<String>,
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        offer: Option<OfferLink>,
    },
    Remove {
        profile: String,
        base: String,
    },
}

pub(crate) fn refusal(field: &str, reason: &str) -> KernelError {
    KernelError::Refused(vec![Refusal {
        field: field.into(),
        backend: String::new(),
        model: String::new(),
        reason: reason.into(),
    }])
}

pub(crate) fn read<T: for<'de> Deserialize<'de>>(state: &mut impl State, key: &str) -> Option<T> {
    state.get(key).and_then(|b| serde_json::from_slice(&b).ok())
}
pub(crate) fn put<T: Serialize>(state: &mut impl State, key: &str, value: &T) {
    let _ = state.put(
        key,
        serde_json::to_vec(value).expect("manager state serializes"),
    );
}
fn pending(state: &mut impl State) -> Vec<String> {
    read(state, "access-asks").unwrap_or_default()
}
pub(crate) fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}
pub(crate) fn valid_profile_text(text: &str) -> bool {
    if text.len() > 1_048_576 || serde_json::from_str::<Strict>(text).is_err() {
        return false;
    }
    let (profile, problems) = parse_profile(text);
    problems.is_empty() && grammar_problems(&profile).is_empty()
}
/// Parse once while refusing duplicate keys at every depth. A duplicate cannot change what the
/// manager says the participant allowed when a later parser picks a different occurrence.
struct Strict(Value);
impl<'de> Deserialize<'de> for Strict {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Strict;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("JSON")
            }
            fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Strict, E> {
                Ok(Strict(Value::Bool(v)))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Strict, E> {
                serde_json::Number::from_f64(v)
                    .map(|n| Strict(Value::Number(n)))
                    .ok_or_else(|| E::custom("non-finite number"))
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Strict, E> {
                Ok(Strict(Value::String(v.into())))
            }
            fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Strict, E> {
                Ok(Strict(Value::String(v)))
            }
            fn visit_none<E: serde::de::Error>(self) -> Result<Strict, E> {
                Ok(Strict(Value::Null))
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Strict, E> {
                Ok(Strict(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Strict, A::Error> {
                let mut out = Vec::new();
                while let Some(v) = seq.next_element::<Strict>()? {
                    out.push(v.0);
                }
                Ok(Strict(Value::Array(out)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Strict, A::Error> {
                let mut out = serde_json::Map::new();
                let mut seen = HashSet::new();
                while let Some((key, value)) = map.next_entry::<String, Strict>()? {
                    if !seen.insert(key.clone()) {
                        return Err(serde::de::Error::custom("duplicate key"));
                    }
                    out.insert(key, value.0);
                }
                Ok(Strict(Value::Object(out)))
            }
        }
        deserializer.deserialize_any(V)
    }
}

const FIELDS: &[&str] = &[
    "backend",
    "model",
    "fallback-models",
    "effort",
    "system-prompt",
    "tools",
    "toolsets",
    "delegates",
    "allow",
    "deny",
    "mode",
    "ask",
    "pre-tool-check",
    "subagents",
    "subagent-depth",
    "subagent-limits",
    "skills",
    "sandbox",
    "json-schema",
    "max-turns",
    "max-tool-errors",
    "max-tokens",
    "max-budget-usd",
    "ask-expiry",
    "persist",
    "partial",
];
fn default_field(name: &str) -> Value {
    match name {
        "backend" => Value::String("claude".into()),
        "mode" => Value::String("manual".into()),
        "ask" | "partial" => Value::Bool(false),
        "persist" => Value::Bool(true),
        "subagent-depth" => Value::Number(1.into()),
        "fallback-models" | "tools" | "toolsets" | "delegates" | "allow" | "deny" | "subagents"
        | "skills" => Value::Array(Vec::new()),
        _ => Value::Null,
    }
}
pub(crate) fn field_digests(text: &str) -> Option<BTreeMap<String, String>> {
    let Strict(Value::Object(map)) = serde_json::from_str::<Strict>(text).ok()? else {
        return None;
    };
    Some(
        FIELDS
            .iter()
            .map(|name| {
                let value = map
                    .get(*name)
                    .cloned()
                    .unwrap_or_else(|| default_field(name));
                (
                    name.to_string(),
                    access::digest(serde_json::to_string(&value).unwrap().as_bytes()),
                )
            })
            .collect(),
    )
}
pub(crate) fn changed_fields(
    old: Option<&BTreeMap<String, String>>,
    new: &BTreeMap<String, String>,
) -> Vec<String> {
    FIELDS
        .iter()
        .filter(|name| old.is_none_or(|m| m.get(**name) != new.get(**name)))
        .map(|name| name.to_string())
        .collect()
}
fn code(connection: &str) -> String {
    const B32: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let hash = Sha256::digest(connection.as_bytes());
    let mut acc = 0u32;
    let mut bits = 0u32;
    let mut out = String::new();
    for byte in hash {
        acc = (acc << 8) | u32::from(byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(B32[((acc >> bits) & 31) as usize] as char);
            if out.len() == 6 {
                return out;
            }
        }
    }
    out
}
fn shown(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if (' '..='~').contains(&c) {
                c.to_string()
            } else {
                format!("\\u{{{:04X}}}", c as u32)
            }
        })
        .collect()
}
pub(crate) fn reach(text: &str, client: Option<&str>) -> Result<String, KernelError> {
    let (p, problems) = parse_profile(text);
    if !problems.is_empty() {
        return Err(refusal("profile", "invalid profile"));
    }
    let mut lines = Vec::new();
    let backend = p.backend.as_deref().unwrap_or("claude");
    let model = p
        .model
        .as_deref()
        .map(shown)
        .unwrap_or_else(|| "its default model".into());
    lines.push(match backend {
        "openai" => format!("ChatGPT, model {model}"),
        _ => {
            if p.model.is_some() {
                format!("Claude Code, model {model}")
            } else {
                "Claude Code, its default model".into()
            }
        }
    });
    if backend == "claude" {
        if p.tools.iter().any(|t| {
            ["Read", "Write", "Edit", "Glob", "Grep", "NotebookEdit"].contains(&t.as_str())
        }) {
            lines.push("Can read and change files in the folders you choose".into());
        }
        let shell = p
            .tools
            .iter()
            .any(|t| ["Bash", "WebFetch", "WebSearch"].contains(&t.as_str()));
        if shell {
            lines.push("Can run programs and use the web, inside Claude's sandbox".into());
            let sandbox = p
                .sandbox
                .as_ref()
                .and_then(Value::as_object)
                .ok_or_else(|| refusal("profile", "shell tools need an enabled sandbox"))?;
            if sandbox.get("enabled").and_then(Value::as_bool) != Some(true)
                || sandbox
                    .keys()
                    .any(|k| !["enabled", "network"].contains(&k.as_str()))
            {
                return Err(refusal("profile", "shell tools need an enabled sandbox"));
            }
            if sandbox
                .get("network")
                .and_then(Value::as_object)
                .is_some_and(|o| o.keys().any(|k| k != "allowedDomains"))
            {
                return Err(refusal("profile", "invalid network allowlist"));
            }
            let domains = p
                .sandbox
                .as_ref()
                .and_then(|v| v.pointer("/network/allowedDomains"));
            let domains: Vec<String> = match domains {
                None => Vec::new(),
                Some(Value::Array(a)) => a
                    .iter()
                    .map(|v| v.as_str().map(str::to_owned))
                    .collect::<Option<Vec<_>>>()
                    .ok_or_else(|| refusal("profile", "invalid network allowlist"))?,
                _ => return Err(refusal("profile", "invalid network allowlist")),
            };
            if domains.iter().any(|d| {
                d.is_empty()
                    || d.len() > 253
                    || !d
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-')
            }) {
                return Err(refusal("profile", "invalid network allowlist"));
            }
            if domains.is_empty() {
                lines.push("Programs have no network".into());
            } else {
                let mut domains: Vec<String> = domains.iter().map(|d| shown(d)).collect();
                domains.sort();
                lines.push(format!("Programs may reach: {}", domains.join(", ")));
            }
        }
        if p.tools.is_empty() {
            lines.push("No built-in tools".into());
        }
    }
    let list = |name: &str, values: &[String]| {
        let mut values: Vec<String> = values.iter().map(|s| shown(s)).collect();
        values.sort();
        (!values.is_empty()).then(|| format!("{name}: {}", values.join(", ")))
    };
    if let Some(line) = list("Tools", &p.tools) {
        lines.push(line);
    }
    if let Some(line) = list("Without asking", &p.allow) {
        lines.push(line);
    }
    lines.push(if p.ask == Some(true) {
        match client {
            Some("page") => "Asks you, in the agent console, before each other tool call".into(),
            Some(c) => format!(
                "Asks {} before each other tool call; {} may allow any of them",
                shown(c),
                shown(c)
            ),
            None => "Asks the mod that starts it before each other tool call".into(),
        }
    } else {
        "Refuses each tool call no rule allows".into()
    });
    if let Some(mode) = p.mode.as_deref().filter(|m| *m != "manual") {
        let meaning = match mode {
            "accept-edits" => "file edits run without asking",
            "plan" => "plans only, changes nothing",
            "dont-ask" => "refuses what no rule allows, asks nobody",
            _ => "",
        };
        lines.push(format!("Mode: {}, {meaning}", shown(mode)));
    }
    if let Some(line) = list("Toolsets", &p.toolsets) {
        lines.push(line);
    }
    if let Some(line) = list(
        "Skills",
        &p.skills.iter().map(|s| s.name.clone()).collect::<Vec<_>>(),
    ) {
        lines.push(line);
    }
    if let Some(line) = list("Delegates", &p.delegates) {
        lines.push(format!("{line} (each narrower than this profile)"));
    }
    let mut limits = Vec::new();
    if let Some(v) = p.max_turns {
        limits.push(format!("max-turns {v}"));
    }
    if let Some(v) = p.max_tool_errors {
        limits.push(format!("max-tool-errors {v}"));
    }
    if let Some(v) = p.max_tokens {
        limits.push(format!("max-tokens {v}"));
    }
    if let Some(v) = p.max_budget_usd {
        limits.push(format!("max-budget-usd {v}"));
    }
    if let Some(v) = p.ask_expiry {
        limits.push(format!("ask-expiry {v}"));
    }
    if let Some(v) = p.subagent_depth {
        limits.push(format!("subagent-depth {v}"));
    }
    if let Some(v) = p.subagent_limits {
        limits.push(format!("subagent-limits {}", shown(&v.to_string())));
    }
    if !limits.is_empty() {
        lines.push(format!("Limits: {}", limits.join(", ")));
    }
    let text = lines.join("\n");
    if text.len() > 4096 {
        return Err(refusal("profile", "too wide to show"));
    }
    Ok(text)
}

impl<S: State, H: Host> Kernel<S, H> {
    pub(crate) fn cleanup_access_after_restart(&mut self) {
        if let Some(streams) = read::<Vec<String>>(&mut self.state, "access-asks") {
            for stream in streams {
                self.state.delete(&format!("access-ask/{stream}"));
                self.state.delete(&format!("stream/{stream}"));
            }
            self.state.delete("access-asks");
        }
        for key in ["manage-ask", "manage-lease"] {
            if let Some(bound) = read::<BoundStream>(&mut self.state, key) {
                self.state.delete(&format!("stream/{}", bound.stream));
                self.state.delete(key);
            }
        }
    }
    fn connection_for(&mut self, caller: &str) -> Result<String, KernelError> {
        if caller != "page" {
            return Err(KernelError::NotOwner);
        }
        self.host.connection().ok_or(KernelError::NotOwner)
    }
    pub(crate) fn lease(&mut self, caller: &str) -> Result<BoundStream, KernelError> {
        let connection = self.connection_for(caller)?;
        let lease: BoundStream =
            read(&mut self.state, "manage-lease").ok_or(KernelError::NotOwner)?;
        if lease.connection != connection {
            return Err(KernelError::NotOwner);
        }
        self.begin()?;
        if read::<BoundStream>(&mut self.state, "manage-lease").as_ref() != Some(&lease) {
            return Err(KernelError::NotOwner);
        }
        Ok(lease)
    }
    pub(crate) fn question(&mut self, q: AccessQuestion, data: Pending) -> Result<(), KernelError> {
        let mut ids = pending(&mut self.state);
        if ids.len() >= 2 {
            return Err(KernelError::Busy("2 profile questions".into()));
        }
        let stream = self
            .host
            .access_ask(q)
            .map_err(|e| KernelError::Backend(e.code().into()))?;
        put(&mut self.state, &format!("access-ask/{stream}"), &data);
        put(
            &mut self.state,
            &format!("stream/{stream}"),
            &serde_json::json!({"kind":"access"}),
        );
        ids.push(stream);
        put(&mut self.state, "access-asks", &ids);
        Ok(())
    }
    pub(crate) fn cancel_question(&mut self, stream: &str) {
        self.host.cancel_stream(stream);
        self.state.delete(&format!("access-ask/{stream}"));
        self.state.delete(&format!("stream/{stream}"));
        let mut ids = pending(&mut self.state);
        ids.retain(|id| id != stream);
        put(&mut self.state, "access-asks", &ids);
    }
    fn questions_for(&mut self, client: &str, profile: &str) -> Vec<(String, Pending)> {
        pending(&mut self.state)
            .into_iter()
            .filter_map(|id| {
                let q: Pending = read(&mut self.state, &format!("access-ask/{id}"))?;
                match &q {
                    Pending::Allow {
                        client: c,
                        profile: p,
                        ..
                    } if c == client && p == profile => Some((id, q)),
                    _ => None,
                }
            })
            .collect()
    }
    fn names_profile(&mut self, profile: &str, except: Option<&str>) -> bool {
        let mut clients: Vec<String> = read(&mut self.state, "clients").unwrap_or_default();
        clients.push("page".into());
        clients.iter().any(|client| {
            access::entries(&mut self.state, client)
                .is_some_and(|entries| entries.iter().any(|e| e.profile == profile))
        }) || pending(&mut self.state)
            .into_iter()
            .filter(|id| Some(id.as_str()) != except)
            .any(|id| {
                read::<Pending>(&mut self.state, &format!("access-ask/{id}")).is_some_and(|q| {
                    match q {
                        Pending::Allow { profile: p, .. }
                        | Pending::Store { profile: p, .. }
                        | Pending::Remove { profile: p, .. } => p == profile,
                    }
                })
            })
            || clients
                .iter()
                .any(|client| crate::withdraw::has_withdrawing(&mut self.state, client, profile))
    }
    pub fn manage_open(&mut self, caller: &str) -> Result<String, KernelError> {
        let connection = self.connection_for(caller)?;
        self.begin()?;
        let existing: Option<BoundStream> = read(&mut self.state, "manage-ask");
        let _lease: Option<BoundStream> = read(&mut self.state, "manage-lease");
        if existing
            .as_ref()
            .is_some_and(|a| a.connection == connection)
        {
            return Ok(code(&connection));
        }
        let stream = self
            .host
            .access_ask(AccessQuestion::Manage)
            .map_err(|e| KernelError::Backend(e.code().into()))?;
        if let Some(old) = existing {
            self.host.cancel_stream(&old.stream);
            self.state.delete(&format!("stream/{}", old.stream));
        }
        let pairing = code(&connection);
        put(
            &mut self.state,
            "manage-ask",
            &BoundStream {
                stream: stream.clone(),
                connection,
            },
        );
        put(
            &mut self.state,
            &format!("stream/{stream}"),
            &serde_json::json!({"kind":"access"}),
        );
        Ok(pairing)
    }
    pub fn manage_close(&mut self, caller: &str) -> Result<(), KernelError> {
        let lease = self.lease(caller)?;
        self.host.cancel_stream(&lease.stream);
        self.state.delete("manage-lease");
        self.state.delete(&format!("stream/{}", lease.stream));
        Ok(())
    }
    pub fn manage_profiles(
        &mut self,
        caller: &str,
        after: Option<&str>,
        max: u32,
    ) -> Result<Vec<StoredProfile>, KernelError> {
        self.lease(caller)?;
        if max == 0 {
            return Err(refusal("max", "must be positive"));
        }
        let index = access::index(&mut self.state);
        let start = after
            .and_then(|name| index.iter().position(|p| p.name == name))
            .map_or(0, |i| i + 1);
        Ok(index
            .into_iter()
            .skip(start)
            .take(max.min(64) as usize)
            .map(|p| StoredProfile {
                name: p.name,
                digest: p.digest,
                rev: p.rev,
                bytes: p.bytes,
                offer: p.offer.map(|o| o.name),
                stale: p.stale,
            })
            .collect())
    }
    pub fn manage_profile(&mut self, caller: &str, name: &str) -> Result<String, KernelError> {
        self.lease(caller)?;
        if !access::index(&mut self.state)
            .iter()
            .any(|p| p.name == name)
        {
            return Err(refusal("profile", "no such profile"));
        }
        self.state
            .get(&format!("profile/{name}"))
            .and_then(|b| String::from_utf8(b).ok())
            .ok_or_else(|| refusal("profile", "no such profile"))
    }
    pub fn manage_put_profile(
        &mut self,
        caller: &str,
        name: &str,
        profile: &str,
    ) -> Result<String, KernelError> {
        self.lease(caller)?;
        if !valid_name(name) {
            return Err(refusal("name", "invalid profile name"));
        }
        if profile.len() > 1_048_576 {
            return Err(KernelError::TooLarge(1_048_576));
        }
        let _: Strict = serde_json::from_str(profile)
            .map_err(|_| refusal("profile", "invalid or duplicate JSON key"))?;
        let (parsed, problems) = parse_profile(profile);
        if !problems.is_empty() || !grammar_problems(&parsed).is_empty() {
            return Err(refusal("profile", "invalid profile"));
        }
        let index = access::index_checked(&mut self.state)
            .ok_or_else(|| refusal("profile", "unreadable profile index"))?;
        let old = index.iter().find(|p| p.name == name);
        if old.is_none() && index.len() >= 256 {
            return Err(refusal("name", "256 profiles"));
        }
        if old.is_some_and(|p| p.rev == u32::MAX) {
            return Err(refusal("profile", "version limit reached"));
        }
        let digest = access::digest(profile.as_bytes());
        if old.is_some_and(|p| p.digest == digest) {
            return Ok(digest);
        }
        let next_fields =
            field_digests(profile).ok_or_else(|| refusal("profile", "invalid profile"))?;
        let previous = old
            .and_then(|_| self.state.get(&format!("profile/{name}")))
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .and_then(|s| field_digests(&s));
        let changed = changed_fields(previous.as_ref(), &next_fields);
        let mut text = reach(profile, None)?;
        if old.is_some() {
            text.push_str(&format!("\nChanged: {}", changed.join(", ")));
        } else {
            text.push_str("\nNew profile");
        }
        if text.len() > 4096 {
            return Err(refusal("profile", "too wide to show"));
        }
        self.question(
            AccessQuestion::Store {
                profile: name.into(),
                text,
            },
            Pending::Store {
                profile: name.into(),
                digest: digest.clone(),
                base: old.map(|p| p.digest.clone()),
                text: profile.into(),
                offer: None,
            },
        )?;
        Ok(digest)
    }
    pub fn manage_delete_profile(&mut self, caller: &str, name: &str) -> Result<(), KernelError> {
        self.lease(caller)?;
        let index = access::index_checked(&mut self.state)
            .ok_or_else(|| refusal("profile", "unreadable profile index"))?;
        let row = index
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| refusal("profile", "no such profile"))?;
        let base = row.digest.clone();
        if self.names_profile(name, None) {
            return Err(refusal("profile", "allowed or asked"));
        }
        self.question(
            AccessQuestion::Remove {
                profile: name.into(),
            },
            Pending::Remove {
                profile: name.into(),
                base,
            },
        )
    }
    pub fn manage_clients(
        &mut self,
        caller: &str,
        after: Option<&str>,
        max: u32,
    ) -> Result<Vec<ClientInfo>, KernelError> {
        self.lease(caller)?;
        if max == 0 {
            return Err(refusal("max", "must be positive"));
        }
        let mut clients = vec!["page".to_string()];
        clients.extend(read::<Vec<String>>(&mut self.state, "clients").unwrap_or_default());
        let start = after
            .and_then(|name| clients.iter().position(|c| c == name))
            .map_or(0, |i| i + 1);
        let index = access::index(&mut self.state);
        let ids = pending(&mut self.state);
        let mut out = Vec::new();
        for client in clients.into_iter().skip(start).take(max.min(16) as usize) {
            let allowed = access::entries(&mut self.state, &client)
                .unwrap_or_default()
                .into_iter()
                .map(|entry| {
                    let standing = match index.iter().find(|p| p.name == entry.profile) {
                        Some(p) if p.stale => Standing::Stale,
                        _ if access::holds(&entry, &index) => Standing::Current,
                        _ => Standing::Changed,
                    };
                    Allowed {
                        profile: entry.profile,
                        digest: entry.digest,
                        standing,
                    }
                })
                .collect();
            let asking = ids
                .iter()
                .filter(|id| {
                    read::<Pending>(&mut self.state, &format!("access-ask/{id}")).is_some_and(
                        |q| matches!(q, Pending::Allow { client: c, .. } if c == client),
                    )
                })
                .cloned()
                .collect();
            out.push(ClientInfo {
                identity: client,
                allowed,
                asking,
            });
        }
        Ok(out)
    }
    pub fn manage_forget(&mut self, caller: &str, client: &str) -> Result<(), KernelError> {
        self.lease(caller)?;
        let mut clients: Vec<String> = read(&mut self.state, "clients").unwrap_or_default();
        if !clients.contains(&client.to_string()) {
            return Err(refusal("client", "not a client of the kernel"));
        }
        let asked = pending(&mut self.state).into_iter().any(|id| {
            read::<Pending>(&mut self.state, &format!("access-ask/{id}"))
                .is_some_and(|q| matches!(q, Pending::Allow { client: c, .. } if c == client))
        });
        let withdrawing = crate::withdraw::has_client_withdrawing(&mut self.state, client);
        if !access::entries(&mut self.state, client)
            .unwrap_or_default()
            .is_empty()
            || asked
            || withdrawing
        {
            return Err(refusal("client", "has access or a question"));
        }
        clients.retain(|c| c != client);
        put(&mut self.state, "clients", &clients);
        Ok(())
    }
    pub fn manage_allow(
        &mut self,
        caller: &str,
        client: &str,
        profile: &str,
    ) -> Result<(), KernelError> {
        self.lease(caller)?;
        let clients: Vec<String> = read(&mut self.state, "clients").unwrap_or_default();
        if client != "page" && !clients.contains(&client.to_string()) {
            return Err(refusal("client", "not a client of the kernel"));
        }
        let index = access::index_checked(&mut self.state)
            .ok_or_else(|| refusal("profile", "unreadable profile index"))?;
        let row = index
            .iter()
            .find(|p| p.name == profile)
            .ok_or_else(|| refusal("profile", "no such profile"))?;
        if row.stale {
            return Err(refusal("profile", "its offer changed"));
        }
        let digest = row.digest.clone();
        let rev = row.rev;
        let entries = access::entries(&mut self.state, client).unwrap_or_default();
        if entries
            .iter()
            .any(|e| e.profile == profile && access::holds(e, &index))
        {
            return Ok(());
        }
        let old = self.questions_for(client, profile);
        if old.iter().any(|(_, q)| matches!(q, Pending::Allow { digest: d, rev: r, .. } if d == &digest && *r == rev)) { return Ok(()); }
        let ids = pending(&mut self.state);
        if ids.len() - old.len() >= 2 {
            return Err(KernelError::Busy("2 profile questions".into()));
        }
        let reserved = ids
            .iter()
            .filter(|id| {
                read::<Pending>(&mut self.state, &format!("access-ask/{id}")).is_some_and(|q| {
                    matches!(q, Pending::Allow { client: c, profile: p, .. }
                if c == client && !entries.iter().any(|e| e.profile == p) && p != profile)
                })
            })
            .count();
        if !entries.iter().any(|e| e.profile == profile) && entries.len() + reserved >= 64 {
            return Err(refusal("client", "64 profiles"));
        }
        let text = self
            .state
            .get(&format!("profile/{profile}"))
            .and_then(|b| String::from_utf8(b).ok())
            .ok_or_else(|| refusal("profile", "no such profile"))?;
        let mut reach = reach(&text, Some(client))?;
        if entries.iter().any(|e| e.profile == profile) {
            let old_fields: Option<BTreeMap<String, String>> = read(
                &mut self.state,
                &format!("access-fields/{client}/{profile}"),
            );
            let new_fields =
                field_digests(&text).ok_or_else(|| refusal("profile", "invalid profile"))?;
            reach.push_str(&format!(
                "\nChanged since the version {} has: {}",
                shown(client),
                changed_fields(old_fields.as_ref(), &new_fields).join(", ")
            ));
            if reach.len() > 4096 {
                return Err(refusal("profile", "too wide to show"));
            }
        }
        let stream = self
            .host
            .access_ask(AccessQuestion::Allow {
                client: client.into(),
                profile: profile.into(),
                reach,
            })
            .map_err(|e| KernelError::Backend(e.code().into()))?;
        for (id, _) in old {
            self.cancel_question(&id);
        }
        put(
            &mut self.state,
            &format!("access-ask/{stream}"),
            &Pending::Allow {
                client: client.into(),
                profile: profile.into(),
                digest,
                rev,
            },
        );
        put(
            &mut self.state,
            &format!("stream/{stream}"),
            &serde_json::json!({"kind":"access"}),
        );
        let mut ids = pending(&mut self.state);
        ids.push(stream);
        put(&mut self.state, "access-asks", &ids);
        Ok(())
    }
    pub fn manage_on_item(&mut self, stream: &str, item: &Value) {
        if read::<Value>(&mut self.state, &format!("stream/{stream}"))
            .as_ref()
            .and_then(|v| v.get("kind"))
            .and_then(Value::as_str)
            != Some("access")
        {
            return;
        }
        if item != &serde_json::json!({"type":"granted"}) {
            return;
        }
        if let Some(ask) = read::<BoundStream>(&mut self.state, "manage-ask") {
            if ask.stream == stream {
                if let Some(old) = read::<BoundStream>(&mut self.state, "manage-lease") {
                    self.host.cancel_stream(&old.stream);
                    self.state.delete(&format!("stream/{}", old.stream));
                }
                self.state.delete("manage-ask");
                put(&mut self.state, "manage-lease", &ask);
                return;
            }
        }
        let Some(question) = read::<Pending>(&mut self.state, &format!("access-ask/{stream}"))
        else {
            return;
        };
        match question {
            Pending::Store {
                profile,
                digest,
                base,
                text,
                offer,
            } => {
                let Some(mut index) = access::index_checked(&mut self.state) else {
                    return;
                };
                let old = index.iter().position(|p| p.name == profile);
                if old.as_ref().map(|i| index[*i].digest.clone()) != base
                    || (old.is_none() && index.len() >= 256)
                {
                    return;
                }
                let rev = match old {
                    Some(i) => match index[i].rev.checked_add(1) {
                        Some(rev) => rev,
                        None => return,
                    },
                    None => 1,
                };
                let row = ProfileIndexEntry {
                    name: profile.clone(),
                    digest,
                    rev,
                    bytes: text.len() as u32,
                    offer,
                    stale: false,
                    gone: false,
                };
                if let Some(i) = old {
                    index[i] = row;
                } else {
                    index.push(row);
                }
                let _ = self
                    .state
                    .put(&format!("profile/{profile}"), text.into_bytes());
                put(&mut self.state, "profiles", &index);
            }
            Pending::Allow {
                client,
                profile,
                digest,
                rev,
            } => {
                let Some(index) = access::index_checked(&mut self.state) else {
                    return;
                };
                if !index
                    .iter()
                    .any(|p| p.name == profile && p.digest == digest && p.rev == rev && !p.stale)
                {
                    return;
                }
                let Some(raw) = self.state.get(&format!("profile/{profile}")) else {
                    return;
                };
                if access::digest(&raw) != digest {
                    return;
                }
                let Some(fields) = String::from_utf8(raw).ok().and_then(|s| field_digests(&s))
                else {
                    return;
                };
                let mut entries = access::entries(&mut self.state, &client).unwrap_or_default();
                if let Some(e) = entries.iter_mut().find(|e| e.profile == profile) {
                    *e = AccessEntry {
                        profile: profile.clone(),
                        digest,
                        rev,
                    };
                } else if entries.len() < 64 {
                    entries.push(AccessEntry {
                        profile: profile.clone(),
                        digest,
                        rev,
                    });
                } else {
                    return;
                }
                put(&mut self.state, &format!("access/{client}"), &entries);
                put(
                    &mut self.state,
                    &format!("access-fields/{client}/{profile}"),
                    &fields,
                );
            }
            Pending::Remove { profile, base } => {
                let Some(mut index) = access::index_checked(&mut self.state) else {
                    return;
                };
                let Some(i) = index
                    .iter()
                    .position(|p| p.name == profile && p.digest == base)
                else {
                    return;
                };
                if self.names_profile(&profile, Some(stream)) {
                    return;
                }
                index.remove(i);
                self.state.delete(&format!("profile/{profile}"));
                put(&mut self.state, "profiles", &index);
            }
        }
        self.state.delete(&format!("access-ask/{stream}"));
    }
    pub fn manage_on_ended(&mut self, stream: &str) {
        if read::<Value>(&mut self.state, &format!("stream/{stream}"))
            .as_ref()
            .and_then(|v| v.get("kind"))
            .and_then(Value::as_str)
            != Some("access")
        {
            return;
        }
        if read::<BoundStream>(&mut self.state, "manage-ask").is_some_and(|a| a.stream == stream) {
            self.state.delete("manage-ask");
        }
        if read::<BoundStream>(&mut self.state, "manage-lease").is_some_and(|a| a.stream == stream)
        {
            self.state.delete("manage-lease");
        }
        let mut ids = pending(&mut self.state);
        if ids.contains(&stream.to_string()) {
            ids.retain(|id| id != stream);
            put(&mut self.state, "access-asks", &ids);
            self.state.delete(&format!("access-ask/{stream}"));
        }
        self.state.delete(&format!("stream/{stream}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FakeHost, MemState};
    #[test]
    fn ac1_only_the_answer_opens_a_connection_bound_lease() {
        let mut k = Kernel::new(
            MemState::new(),
            FakeHost {
                connection_id: Some("k1".into()),
                ..FakeHost::default()
            },
        );
        assert_eq!(
            k.manage_profiles("page", None, 10).unwrap_err().code(),
            "not-owner"
        );
        assert_eq!(
            k.manage_open("local/client").unwrap_err().code(),
            "not-owner"
        );
        let code = k.manage_open("page").unwrap();
        assert_eq!(code.len(), 6);
        assert_eq!(
            k.manage_profiles("page", None, 10).unwrap_err().code(),
            "not-owner"
        );
        k.manage_on_item("t1", &serde_json::json!({"type":"granted"}));
        assert!(k.manage_profiles("page", None, 10).is_ok());
        k.host.connection_id = Some("k2".into());
        assert_eq!(
            k.manage_profiles("page", None, 10).unwrap_err().code(),
            "not-owner"
        );
    }
    #[test]
    fn ac2_store_and_allow_wait_for_exact_host_answer() {
        let mut k = Kernel::new(
            MemState::new(),
            FakeHost {
                connection_id: Some("k1".into()),
                ..FakeHost::default()
            },
        );
        k.manage_open("page").unwrap();
        k.manage_on_item("t1", &serde_json::json!({"type":"granted"}));
        let profile = r#"{"backend":"claude","tools":[]}"#;
        let digest = k.manage_put_profile("page", "p", profile).unwrap();
        assert!(k.state.get("profile/p").is_none());
        k.manage_on_item("t2", &serde_json::json!({"type":"refused"}));
        assert!(k.state.get("profile/p").is_none());
        k.manage_on_item("t2", &serde_json::json!({"type":"granted"}));
        assert_eq!(k.state.get("profile/p").unwrap(), profile.as_bytes());
        assert_eq!(access::index(&mut k.state)[0].digest, digest);
        k.state.put("clients", br#"["local/c"]"#.to_vec()).unwrap();
        k.manage_allow("page", "local/c", "p").unwrap();
        assert!(k.state.get("access/local/c").is_none());
        k.manage_on_item("t3", &serde_json::json!({"type":"granted"}));
        assert_eq!(
            access::entries(&mut k.state, "local/c").unwrap()[0].digest,
            digest
        );
        assert!(k.state.get("access-fields/local/c/p").is_some());
    }
    #[test]
    fn ac2_duplicate_nested_key_is_refused_without_question() {
        let mut k = Kernel::new(
            MemState::new(),
            FakeHost {
                connection_id: Some("k1".into()),
                ..FakeHost::default()
            },
        );
        k.manage_open("page").unwrap();
        k.manage_on_item("t1", &serde_json::json!({"type":"granted"}));
        assert_eq!(
            k.manage_put_profile("page", "p", r#"{"sandbox":{"network":[],"network":[]}}"#)
                .unwrap_err()
                .code(),
            "refused"
        );
        assert!(!k
            .host
            .calls
            .iter()
            .any(|c| matches!(c, crate::HostCall::AccessAsk { stream, .. } if stream == "t2")));
    }
    #[test]
    fn ac2_restart_drops_question_and_lease_before_access_can_resume() {
        let mut k = Kernel::new(
            MemState::new(),
            FakeHost {
                connection_id: Some("k1".into()),
                ..FakeHost::default()
            },
        );
        k.manage_open("page").unwrap();
        k.manage_on_item("t1", &serde_json::json!({"type":"granted"}));
        k.manage_put_profile("page", "p", "{}").unwrap();
        assert!(k.state.get("access-ask/t2").is_some());
        k.host.generation += 1;
        assert_eq!(
            k.manage_profiles("page", None, 1).unwrap_err().code(),
            "not-owner"
        );
        assert!(k.state.get("manage-lease").is_none());
        assert!(k.state.get("access-ask/t2").is_none());
    }
    #[test]
    fn ac3_reach_names_shell_network_and_rejects_escape_settings() {
        let safe = r#"{"backend":"claude","tools":["Bash"],"sandbox":{"enabled":true,"network":{"allowedDomains":["example.com"]}}}"#;
        assert!(reach(safe, Some("local/c"))
            .unwrap()
            .contains("Programs may reach: example.com"));
        let unsafe_text =
            r#"{"tools":["Bash"],"sandbox":{"enabled":true,"allowUnsandboxedCommands":true}}"#;
        assert_eq!(reach(unsafe_text, None).unwrap_err().code(), "refused");
    }
    #[test]
    fn ac3_field_digests_use_top_level_defaults_only() {
        let old = field_digests(r#"{"model":"a","subagent-limits":{"live":2}}"#).unwrap();
        let default_written =
            field_digests(r#"{"backend":"claude","model":"b","subagent-limits":{"live":2}}"#)
                .unwrap();
        assert_eq!(changed_fields(Some(&old), &default_written), vec!["model"]);
        let nested =
            field_digests(r#"{"model":"a","subagent-limits":{"live":2,"depth":1}}"#).unwrap();
        assert_eq!(changed_fields(Some(&old), &nested), vec!["subagent-limits"]);
    }
    #[test]
    fn val_field_digest_is_rfc8785_key_order_independent() {
        let a = field_digests(r#"{"subagent-limits":{"live":2,"depth":1}}"#).unwrap();
        let b = field_digests(r#"{"subagent-limits":{"depth":1,"live":2}}"#).unwrap();
        assert!(changed_fields(Some(&a), &b).is_empty());
    }
    #[test]
    fn ac2_remove_requires_a_fresh_answer_and_no_grant() {
        let mut k = Kernel::new(
            MemState::new(),
            FakeHost {
                connection_id: Some("k1".into()),
                ..FakeHost::default()
            },
        );
        k.manage_open("page").unwrap();
        k.manage_on_item("t1", &serde_json::json!({"type":"granted"}));
        k.manage_put_profile("page", "p", "{}").unwrap();
        k.manage_on_item("t2", &serde_json::json!({"type":"granted"}));
        k.manage_on_ended("t2");
        k.manage_delete_profile("page", "p").unwrap();
        assert!(k.state.get("profile/p").is_some());
        k.manage_on_item("t3", &serde_json::json!({"type":"granted"}));
        assert!(k.state.get("profile/p").is_none());
    }
    #[test]
    fn ac2_superseded_allow_reuses_the_reserved_slot() {
        let mut k = Kernel::new(
            MemState::new(),
            FakeHost {
                connection_id: Some("k1".into()),
                ..FakeHost::default()
            },
        );
        k.manage_open("page").unwrap();
        k.manage_on_item("t1", &serde_json::json!({"type":"granted"}));
        k.manage_put_profile("page", "p", r#"{"model":"a"}"#)
            .unwrap();
        k.manage_on_item("t2", &serde_json::json!({"type":"granted"}));
        k.manage_on_ended("t2");
        k.state.put("clients", br#"["local/c"]"#.to_vec()).unwrap();
        k.manage_allow("page", "local/c", "p").unwrap();
        k.manage_put_profile("page", "p", r#"{"model":"b"}"#)
            .unwrap();
        k.manage_on_item("t4", &serde_json::json!({"type":"granted"}));
        k.manage_allow("page", "local/c", "p").unwrap();
        assert_eq!(pending(&mut k.state), vec!["t4", "t5"]);
        assert!(k.state.get("access-ask/t3").is_none());
        k.manage_on_item("t3", &serde_json::json!({"type":"granted"}));
        assert!(k.state.get("access/local/c").is_none());
        k.manage_on_item("t5", &serde_json::json!({"type":"granted"}));
        assert_eq!(access::entries(&mut k.state, "local/c").unwrap()[0].rev, 2);
    }
    #[test]
    fn ac1_access_sink_ignores_other_stream_kinds() {
        let mut k = Kernel::new(MemState::new(), FakeHost::default());
        let value = serde_json::json!({"kind":"provider","session":"s1"});
        put(&mut k.state, "stream/t1", &value);
        k.manage_on_item("t1", &serde_json::json!({"type":"granted"}));
        k.manage_on_ended("t1");
        assert_eq!(read::<Value>(&mut k.state, "stream/t1"), Some(value));
    }
    #[test]
    fn ac1_pairing_code_is_six_rfc4648_base32_characters_of_connection_hash() {
        assert_eq!(code("k1"), "NK47D2");
        assert_eq!(code("k2"), "AFPX42");
        assert_eq!(code("k3"), "F5IFFS");
    }
    #[test]
    fn ac2_pending_allow_reserves_the_sixty_fourth_entry() {
        let mut k = Kernel::new(
            MemState::new(),
            FakeHost {
                connection_id: Some("k1".into()),
                ..FakeHost::default()
            },
        );
        k.manage_open("page").unwrap();
        k.manage_on_item("t1", &serde_json::json!({"type":"granted"}));
        k.state.put("clients", br#"["local/c"]"#.to_vec()).unwrap();
        let text = b"{}";
        let digest = access::digest(text);
        let index: Vec<_> = (0..65)
            .map(|n| ProfileIndexEntry {
                name: format!("p{n}"),
                digest: digest.clone(),
                rev: 1,
                bytes: 2,
                offer: None,
                stale: false,
                gone: false,
            })
            .collect();
        put(&mut k.state, "profiles", &index);
        for n in 0..65 {
            k.state
                .put(&format!("profile/p{n}"), text.to_vec())
                .unwrap();
        }
        let entries: Vec<_> = (0..63)
            .map(|n| AccessEntry {
                profile: format!("p{n}"),
                digest: digest.clone(),
                rev: 1,
            })
            .collect();
        put(&mut k.state, "access/local/c", &entries);
        k.manage_allow("page", "local/c", "p63").unwrap();
        assert_eq!(
            k.manage_allow("page", "local/c", "p64").unwrap_err().code(),
            "refused"
        );
        assert_eq!(pending(&mut k.state), vec!["t2"]);
    }
}
