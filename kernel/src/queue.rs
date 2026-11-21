//! The input queue `queue/<id>`: TECHNICAL 9.3 (Sizes, queue), 9.1.

use crate::error::KernelError;
use crate::state::State;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const QUEUE_INPUTS: usize = 32;
pub const QUEUE_BYTES: usize = 2_097_152;
pub const INPUT_BYTES: usize = 262_144;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Queued {
    pub n: u64,
    /// The input JSON, as a string.
    pub input: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Queue {
    /// The number the next push takes; numbers are never reused.
    pub next: u64,
    pub inputs: Vec<Queued>,
}

impl Default for Queue {
    fn default() -> Self {
        Queue {
            next: 1,
            inputs: Vec::new(),
        }
    }
}

pub fn check_input(input: &str) -> Result<(), KernelError> {
    if input.len() > INPUT_BYTES {
        return Err(KernelError::TooLarge(INPUT_BYTES as u64));
    }
    Ok(())
}

impl Queue {
    pub fn key_of(id: &str) -> String {
        format!("queue/{id}")
    }

    /// An absent (or unreadable) key is the empty queue.
    pub fn load(state: &mut impl State, id: &str) -> Queue {
        state
            .get(&Self::key_of(id))
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    /// An empty queue deletes the key.
    pub fn save(&self, state: &mut impl State, id: &str) -> Result<(), KernelError> {
        let key = Self::key_of(id);
        if self.is_empty() {
            state.delete(&key);
            return Ok(());
        }
        let bytes = serde_json::to_vec(self).expect("a queue serializes");
        state
            .put(&key, bytes)
            .map_err(|t| KernelError::TooLarge(t.0 as u64))
    }

    pub fn push(&mut self, input: String) -> Result<u64, KernelError> {
        let total: usize = self.inputs.iter().map(|q| q.input.len()).sum();
        if self.inputs.len() >= QUEUE_INPUTS || total + input.len() > QUEUE_BYTES {
            return Err(KernelError::QueueFull);
        }
        let n = self.next;
        self.next += 1;
        self.inputs.push(Queued { n, input });
        Ok(n)
    }

    pub fn remove(&mut self, n: u64) -> Result<(), KernelError> {
        match self.inputs.iter().position(|q| q.n == n) {
            Some(i) => {
                self.inputs.remove(i);
                Ok(())
            }
            None => Err(KernelError::UnknownInput(n)),
        }
    }

    pub fn take_oldest(&mut self) -> Option<Queued> {
        let i = self
            .inputs
            .iter()
            .enumerate()
            .min_by_key(|(_, q)| q.n)
            .map(|(i, _)| i)?;
        Some(self.inputs.remove(i))
    }

    pub fn is_empty(&self) -> bool {
        self.inputs.is_empty()
    }

    pub fn push_value(&self) -> Value {
        let inputs: Vec<Value> = self
            .inputs
            .iter()
            .map(|q| json!({"n": q.n, "input": q.input}))
            .collect();
        json!({"type": "queue", "inputs": inputs})
    }
}
