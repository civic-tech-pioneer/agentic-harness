//! Kernel errors and refusals: TECHNICAL 12 (kernel rows), 11 (refusal shape); AGT-F-40.

use crate::profile::Backend;
use serde::{Deserialize, Serialize};
use std::fmt;

/// One refused field: nothing is dropped silently (TECHNICAL 11).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Refusal {
    pub field: String,
    pub backend: String,
    pub model: String,
    pub reason: String,
}

/// A field outside its grammar, before the backend and model are known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    pub field: String,
    pub reason: String,
}

impl Problem {
    pub fn new(field: impl Into<String>, reason: impl Into<String>) -> Self {
        Problem {
            field: field.into(),
            reason: reason.into(),
        }
    }

    pub fn refusal(&self, backend: Backend, model: &str) -> Refusal {
        Refusal {
            field: self.field.clone(),
            backend: backend.as_str().to_string(),
            model: model.to_string(),
            reason: self.reason.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum KernelError {
    NotOwner,
    UnknownSession(String),
    Refused(Vec<Refusal>),
    QueueFull,
    Quota,
    Paused(u64),
    Backend(String),
    UnknownInput(u64),
    UnknownAllow(String),
    Behind(u64),
    TooLarge(u64),
    Busy(String),
}

impl KernelError {
    pub fn code(&self) -> &'static str {
        match self {
            KernelError::NotOwner => "not-owner",
            KernelError::UnknownSession(_) => "unknown-session",
            KernelError::Refused(_) => "refused",
            KernelError::QueueFull => "queue-full",
            KernelError::Quota => "quota",
            KernelError::Paused(_) => "paused",
            KernelError::Backend(_) => "backend",
            KernelError::UnknownInput(_) => "unknown-input",
            KernelError::UnknownAllow(_) => "unknown-allow",
            KernelError::Behind(_) => "behind",
            KernelError::TooLarge(_) => "too-large",
            KernelError::Busy(_) => "busy",
        }
    }
}

impl fmt::Display for KernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KernelError::NotOwner => write!(f, "not-owner: the caller does not own this session"),
            KernelError::UnknownSession(id) => write!(f, "unknown-session: no session {id}"),
            KernelError::Refused(rs) => {
                write!(f, "refused: {} field(s) refused", rs.len())?;
                for (i, r) in rs.iter().enumerate() {
                    let sep = if i == 0 { ": " } else { "; " };
                    write!(
                        f,
                        "{sep}{} ({}, {}): {}",
                        r.field, r.backend, r.model, r.reason
                    )?;
                }
                Ok(())
            }
            KernelError::QueueFull => write!(f, "queue-full: the session's input queue is full"),
            KernelError::Quota => write!(f, "quota: the mod's session quota is used up"),
            KernelError::Paused(t) => write!(f, "paused: the backend is paused until {t}"),
            KernelError::Backend(m) => write!(f, "backend: {m}"),
            KernelError::UnknownInput(n) => write!(f, "unknown-input: no input number {n}"),
            KernelError::UnknownAllow(id) => write!(f, "unknown-allow: no pending ask {id}"),
            KernelError::Behind(n) => write!(f, "behind: {n} entries stored, page with read"),
            KernelError::TooLarge(n) => write!(f, "too-large: over the limit of {n} bytes"),
            KernelError::Busy(cap) => write!(f, "busy: {cap}"),
        }
    }
}

impl std::error::Error for KernelError {}
