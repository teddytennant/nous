//! Command history with persistence and search.
//!
//! Stores recent commands in a ring buffer and supports prefix/substring
//! search for Ctrl-R style reverse search. History can be persisted to
//! a file for cross-session recall.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

/// Maximum number of history entries retained.
const DEFAULT_MAX_ENTRIES: usize = 1000;

/// A single history entry.
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryEntry {
    pub command: String,
    pub timestamp: u64,
}

/// Command history with search and navigation.
#[derive(Debug)]
pub struct History {
    entries: VecDeque<HistoryEntry>,
    max_entries: usize,
    cursor: Option<usize>,
    file_path: Option<PathBuf>,
}

impl History {
    /// Create a new empty history.
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(max_entries.min(1024)),
            max_entries,
            cursor: None,
            file_path: None,
        }
    }

    /// Create a history that persists to a file.
    pub fn with_file(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let mut history = Self::new(DEFAULT_MAX_ENTRIES);
        history.file_path = Some(path.clone());

        if path.exists()
            && let Ok(content) = std::fs::read_to_string(&path)
        {
            for line in content.lines() {
                if let Some((ts_str, cmd)) = line.split_once('\t') {
                    let timestamp = ts_str.parse().unwrap_or(0);
                    if !cmd.is_empty() {
                        history.entries.push_back(HistoryEntry {
                            command: cmd.to_string(),
                            timestamp,
                        });
                    }
                }
            }
            // Trim if loaded more than max
            while history.entries.len() > history.max_entries {
                history.entries.pop_front();
            }
        }

        history
    }

    /// Add a command to history.
    ///
    /// Deduplicates consecutive identical commands. Resets navigation cursor.
    pub fn push(&mut self, command: impl Into<String>) {
        let command = command.into();
        let trimmed = command.trim();
        if trimmed.is_empty() {
            return;
        }

        // Skip if identical to the most recent entry
        if let Some(last) = self.entries.back()
            && last.command == trimmed
        {
            self.cursor = None;
            return;
        }

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        if self.entries.len() >= self.max_entries {
            self.entries.pop_front();
        }

        self.entries.push_back(HistoryEntry {
            command: trimmed.to_string(),
            timestamp,
        });

        self.cursor = None;
    }

    /// Navigate to the previous (older) entry.
    ///
    /// Returns the command text or `None` if at the beginning.
    pub fn previous(&mut self) -> Option<&str> {
        if self.entries.is_empty() {
            return None;
        }

        let new_cursor = match self.cursor {
            None => self.entries.len().checked_sub(1),
            Some(0) => Some(0),
            Some(n) => Some(n - 1),
        };

        self.cursor = new_cursor;
        self.cursor
            .and_then(|idx| self.entries.get(idx))
            .map(|e| e.command.as_str())
    }

    /// Navigate to the next (newer) entry.
    ///
    /// Returns the command text, or `None` if past the end (current input).
    pub fn next_entry(&mut self) -> Option<&str> {
        match self.cursor {
            None => None,
            Some(idx) => {
                if idx + 1 >= self.entries.len() {
                    self.cursor = None;
                    None
                } else {
                    self.cursor = Some(idx + 1);
                    self.entries.get(idx + 1).map(|e| e.command.as_str())
                }
            }
        }
    }

    /// Reset the navigation cursor.
    pub fn reset_cursor(&mut self) {
        self.cursor = None;
    }

    /// Search history for entries matching a prefix (case-insensitive).
    pub fn search_prefix(&self, prefix: &str) -> Vec<&HistoryEntry> {
        let lower = prefix.to_lowercase();
        self.entries
            .iter()
            .rev()
            .filter(|e| e.command.to_lowercase().starts_with(&lower))
            .collect()
    }

    /// Search history for entries containing a substring (case-insensitive).
    pub fn search_substring(&self, query: &str) -> Vec<&HistoryEntry> {
        let lower = query.to_lowercase();
        self.entries
            .iter()
            .rev()
            .filter(|e| e.command.to_lowercase().contains(&lower))
            .collect()
    }

    /// Get all entries, oldest first.
    pub fn entries(&self) -> &VecDeque<HistoryEntry> {
        &self.entries
    }

    /// Number of entries in history.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether history is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Persist history to file, if a file path is configured.
    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = &self.file_path else {
            return Ok(());
        };

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content: String = self
            .entries
            .iter()
            .map(|e| format!("{}\t{}", e.timestamp, e.command))
            .collect::<Vec<_>>()
            .join("\n");

        std::fs::write(path, content)
    }

    /// Get the file path, if configured.
    pub fn file_path(&self) -> Option<&Path> {
        self.file_path.as_deref()
    }

    /// Clear all history entries.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.cursor = None;
    }
}

impl Default for History {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_ENTRIES)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_and_len() {
        let mut h = History::new(10);
        assert!(h.is_empty());

        h.push("/help");
        h.push("/wallet balance");
        assert_eq!(h.len(), 2);
    }

    #[test]
    fn skip_empty_input() {
        let mut h = History::new(10);
        h.push("");
        h.push("   ");
        assert!(h.is_empty());
    }

    #[test]
    fn dedup_consecutive() {
        let mut h = History::new(10);
        h.push("/help");
        h.push("/help");
        h.push("/help");
        assert_eq!(h.len(), 1);
    }

    #[test]
    fn no_dedup_nonconsecutive() {
        let mut h = History::new(10);
        h.push("/help");
        h.push("/wallet balance");
        h.push("/help");
        assert_eq!(h.len(), 3);
    }

    #[test]
    fn max_entries_eviction() {
        let mut h = History::new(3);
        h.push("a");
        h.push("b");
        h.push("c");
        h.push("d");

        assert_eq!(h.len(), 3);
        assert_eq!(h.entries[0].command, "b");
        assert_eq!(h.entries[2].command, "d");
    }

    #[test]
    fn navigate_previous() {
        let mut h = History::new(10);
        h.push("first");
        h.push("second");
        h.push("third");

        assert_eq!(h.previous(), Some("third"));
        assert_eq!(h.previous(), Some("second"));
        assert_eq!(h.previous(), Some("first"));
        assert_eq!(h.previous(), Some("first")); // stays at beginning
    }

    #[test]
    fn navigate_next() {
        let mut h = History::new(10);
        h.push("first");
        h.push("second");
        h.push("third");

        h.previous(); // third
        h.previous(); // second
        h.previous(); // first

        assert_eq!(h.next_entry(), Some("second"));
        assert_eq!(h.next_entry(), Some("third"));
        assert_eq!(h.next_entry(), None); // past end = current input
    }

    #[test]
    fn next_without_previous_returns_none() {
        let mut h = History::new(10);
        h.push("test");
        assert!(h.next_entry().is_none());
    }

    #[test]
    fn reset_cursor() {
        let mut h = History::new(10);
        h.push("first");
        h.push("second");

        h.previous();
        h.reset_cursor();

        assert_eq!(h.previous(), Some("second")); // starts from end again
    }

    #[test]
    fn search_prefix() {
        let mut h = History::new(10);
        h.push("/wallet balance");
        h.push("/help");
        h.push("/wallet send");
        h.push("/identity show");

        let results = h.search_prefix("/wallet");
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].command, "/wallet send");
        assert_eq!(results[1].command, "/wallet balance");
    }

    #[test]
    fn search_prefix_case_insensitive() {
        let mut h = History::new(10);
        h.push("/Wallet Balance");
        let results = h.search_prefix("/wallet");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn search_substring() {
        let mut h = History::new(10);
        h.push("/wallet balance");
        h.push("/help wallet");
        h.push("/peer list");

        let results = h.search_substring("wallet");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn search_empty_query() {
        let mut h = History::new(10);
        h.push("/help");
        h.push("/wallet balance");

        let results = h.search_prefix("");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn push_resets_cursor() {
        let mut h = History::new(10);
        h.push("first");
        h.push("second");

        h.previous(); // second
        h.previous(); // first

        h.push("third");

        // After push, cursor is reset — previous goes to latest
        assert_eq!(h.previous(), Some("third"));
    }

    #[test]
    fn persistence_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.txt");

        {
            let mut h = History::with_file(&path);
            h.push("/help");
            h.push("/wallet balance");
            h.push("/identity show");
            h.save().unwrap();
        }

        {
            let h = History::with_file(&path);
            assert_eq!(h.len(), 3);
            assert_eq!(h.entries[0].command, "/help");
            assert_eq!(h.entries[1].command, "/wallet balance");
            assert_eq!(h.entries[2].command, "/identity show");
        }
    }

    #[test]
    fn persistence_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("dir").join("history.txt");

        let mut h = History::with_file(&path);
        h.push("/test");
        h.save().unwrap();

        assert!(path.exists());
    }

    #[test]
    fn load_nonexistent_file() {
        let h = History::with_file("/tmp/nous_test_nonexistent_file.txt");
        assert!(h.is_empty());
    }

    #[test]
    fn clear_history() {
        let mut h = History::new(10);
        h.push("a");
        h.push("b");
        h.clear();
        assert!(h.is_empty());
    }

    #[test]
    fn trims_whitespace_on_push() {
        let mut h = History::new(10);
        h.push("  /help  ");
        assert_eq!(h.entries[0].command, "/help");
    }
}
