//! The kernel's view of host state: TECHNICAL 9.1 (keys), 7 and 2.1 (key-value, 4 MiB per value, no prefix scan).

use std::collections::BTreeMap;

/// The largest value host state takes (host 7).
pub const MAX_VALUE: usize = 4 * 1024 * 1024;

/// A value over `MAX_VALUE`; carries the limit in bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TooLarge(pub usize);

/// Host state as the kernel sees it: get, put, delete; nothing else.
pub trait State {
    /// `&mut`: a counting state counts reads.
    fn get(&mut self, key: &str) -> Option<Vec<u8>>;
    fn put(&mut self, key: &str, value: Vec<u8>) -> Result<(), TooLarge>;
    fn delete(&mut self, key: &str);
}

/// What a state was asked to do, so tests can prove what a call touched.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    pub gets: u64,
    pub puts: u64,
    pub deletes: u64,
    pub bytes_read: u64,
    pub bytes_written: u64,
}

/// An in-memory state with call counts and the keys read.
#[derive(Debug, Default)]
pub struct MemState {
    map: BTreeMap<String, Vec<u8>>,
    counts: Counts,
    keys: Vec<String>,
}

impl MemState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn counts(&self) -> Counts {
        self.counts
    }

    /// Zeroes the counts and forgets the keys read.
    pub fn reset_counts(&mut self) {
        self.counts = Counts::default();
        self.keys.clear();
    }

    /// The keys read since the last reset, in order.
    pub fn get_keys(&self) -> &[String] {
        &self.keys
    }

    /// The stored pairs, uncounted.
    pub fn entries(&self) -> impl Iterator<Item = (&String, &Vec<u8>)> {
        self.map.iter()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

impl State for MemState {
    fn get(&mut self, key: &str) -> Option<Vec<u8>> {
        self.counts.gets += 1;
        self.keys.push(key.to_string());
        let v = self.map.get(key).cloned();
        if let Some(v) = &v {
            self.counts.bytes_read += v.len() as u64;
        }
        v
    }

    fn put(&mut self, key: &str, value: Vec<u8>) -> Result<(), TooLarge> {
        if value.len() > MAX_VALUE {
            return Err(TooLarge(MAX_VALUE));
        }
        self.counts.puts += 1;
        self.counts.bytes_written += value.len() as u64;
        self.map.insert(key.to_string(), value);
        Ok(())
    }

    fn delete(&mut self, key: &str) {
        self.counts.deletes += 1;
        self.map.remove(key);
    }
}
