//! Where evaluated lines are kept.
//!
//! [`Calculator`](crate::Calculator) does not know whether its history lives
//! in a `Vec` or in SQLite: it writes through a [`HistoryStore`]. The SQLite
//! implementation is a separate crate (`//core/calc_history_sqlite`), so this
//! one stays dependency-free and compiles for every target a front end needs.

use crate::{Entry, HISTORY_LIMIT};
use std::fmt;

/// A store operation failed. Carries the backend's message, because nothing
/// here can act on a more precise type: a calculator that cannot write its
/// history still has to show the answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreError {
    pub message: String,
}

impl StoreError {
    pub fn new(message: impl Into<String>) -> Self {
        StoreError { message: message.into() }
    }
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for StoreError {}

/// Persistence for evaluated lines.
///
/// Implementations assign the ids, keep at most [`HISTORY_LIMIT`] entries, and
/// never reuse an id: a renderer keys its row views on [`Entry::id`], so an id
/// coming back for a different expression would move the wrong row.
/// `Send` because UniFFI objects are shared across the FFI boundary and may
/// be called from any thread; `rusqlite::Connection` is `Send` on every
/// target, so this costs nothing.
pub trait HistoryStore: fmt::Debug + Send {
    /// Appends one line and returns the id assigned to it, pruning down to
    /// [`HISTORY_LIMIT`].
    fn append(&mut self, expression: &str, result: &str) -> Result<u64, StoreError>;

    /// Every entry still kept, oldest first.
    fn entries(&self) -> Result<Vec<Entry>, StoreError>;

    fn clear(&mut self) -> Result<(), StoreError>;

    /// Where history is going, for the footer caption: "in memory",
    /// "SQLite 3.54.1". Shown by every front end, so it is how a user can
    /// tell a persisted run from a throwaway one.
    fn description(&self) -> String;
}

/// The default store: history lives as long as the process does.
#[derive(Debug, Default)]
pub struct MemoryHistory {
    entries: Vec<Entry>,
    next_id: u64,
    /// Why this is not the store the front end asked for, if it isn't.
    reason: Option<String>,
}

impl MemoryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    /// Memory because the real store could not be opened.
    ///
    /// `reason` travels in [`HistoryStore::description`], so it reaches the
    /// footer of every front end: the calculator keeps working, and the user
    /// can see that this run is not being kept.
    pub fn fallback(reason: impl Into<String>) -> Self {
        MemoryHistory { reason: Some(reason.into()), ..Self::default() }
    }
}

impl HistoryStore for MemoryHistory {
    fn append(&mut self, expression: &str, result: &str) -> Result<u64, StoreError> {
        self.next_id += 1;
        self.entries.push(Entry {
            id: self.next_id,
            expression: expression.to_string(),
            result: result.to_string(),
        });
        if self.entries.len() > HISTORY_LIMIT {
            self.entries.remove(0);
        }
        Ok(self.next_id)
    }

    fn entries(&self) -> Result<Vec<Entry>, StoreError> {
        Ok(self.entries.clone())
    }

    fn clear(&mut self) -> Result<(), StoreError> {
        self.entries.clear();
        Ok(())
    }

    fn description(&self) -> String {
        match &self.reason {
            Some(reason) => format!("memory ({reason})"),
            None => "memory".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fallback_store_says_why_it_is_in_memory() {
        assert_eq!(MemoryHistory::new().description(), "memory");
        assert_eq!(
            MemoryHistory::fallback("unable to open database file").description(),
            "memory (unable to open database file)"
        );
    }

    #[test]
    fn memory_store_appends_prunes_and_clears() {
        let mut store = MemoryHistory::new();
        assert_eq!(store.append("1+1", "2").unwrap(), 1);
        assert_eq!(store.append("2+2", "4").unwrap(), 2);
        assert_eq!(
            store.entries().unwrap(),
            vec![
                Entry { id: 1, expression: "1+1".into(), result: "2".into() },
                Entry { id: 2, expression: "2+2".into(), result: "4".into() },
            ]
        );

        for _ in 0..HISTORY_LIMIT {
            store.append("9", "9").unwrap();
        }
        let entries = store.entries().unwrap();
        assert_eq!(entries.len(), HISTORY_LIMIT);
        assert_eq!(entries.first().unwrap().id, 3, "the oldest entries are dropped");

        store.clear().unwrap();
        assert!(store.entries().unwrap().is_empty());
        assert_eq!(store.append("1", "1").unwrap(), HISTORY_LIMIT as u64 + 3, "ids are not reused");
    }
}
