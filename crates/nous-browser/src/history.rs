use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::resolver::Protocol;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub url: String,
    pub title: String,
    pub protocol: Protocol,
    pub visited_at: DateTime<Utc>,
    pub duration_secs: Option<u64>,
}

impl HistoryEntry {
    pub fn new(url: impl Into<String>, title: impl Into<String>) -> Self {
        let url = url.into();
        let protocol = Protocol::from_url(&url);
        Self {
            url,
            title: title.into(),
            protocol,
            visited_at: Utc::now(),
            duration_secs: None,
        }
    }

    pub fn with_duration(mut self, secs: u64) -> Self {
        self.duration_secs = Some(secs);
        self
    }

    pub fn is_decentralized(&self) -> bool {
        self.protocol.is_decentralized()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowsingHistory {
    entries: Vec<HistoryEntry>,
    max_entries: usize,
}

impl BrowsingHistory {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Vec::new(),
            max_entries,
        }
    }

    pub fn record(&mut self, entry: HistoryEntry) {
        self.entries.push(entry);
        if self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }
    }

    pub fn search(&self, query: &str) -> Vec<&HistoryEntry> {
        let q = query.to_lowercase();
        self.entries
            .iter()
            .filter(|e| e.title.to_lowercase().contains(&q) || e.url.to_lowercase().contains(&q))
            .collect()
    }

    pub fn since(&self, since: DateTime<Utc>) -> Vec<&HistoryEntry> {
        self.entries
            .iter()
            .filter(|e| e.visited_at >= since)
            .collect()
    }

    pub fn by_domain(&self, domain: &str) -> Vec<&HistoryEntry> {
        let d = domain.to_lowercase();
        self.entries
            .iter()
            .filter(|e| e.url.to_lowercase().contains(&d))
            .collect()
    }

    pub fn decentralized_only(&self) -> Vec<&HistoryEntry> {
        self.entries
            .iter()
            .filter(|e| e.is_decentralized())
            .collect()
    }

    pub fn recent(&self, limit: usize) -> Vec<&HistoryEntry> {
        self.entries.iter().rev().take(limit).collect()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn clear_domain(&mut self, domain: &str) {
        let d = domain.to_lowercase();
        self.entries.retain(|e| !e.url.to_lowercase().contains(&d));
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn total_time_secs(&self) -> u64 {
        self.entries.iter().filter_map(|e| e.duration_secs).sum()
    }

    pub fn unique_domains(&self) -> Vec<String> {
        let mut domains: Vec<String> = self
            .entries
            .iter()
            .filter_map(|e| extract_domain(&e.url))
            .collect();
        domains.sort();
        domains.dedup();
        domains
    }
}

impl Default for BrowsingHistory {
    fn default() -> Self {
        Self::new(10_000)
    }
}

fn extract_domain(url: &str) -> Option<String> {
    let without_proto = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .or_else(|| url.strip_prefix("ipfs://"))
        .or_else(|| url.strip_prefix("ipns://"))
        .or_else(|| url.strip_prefix("ar://"))
        .or_else(|| url.strip_prefix("nous://"))
        .unwrap_or(url);

    let domain = without_proto.split('/').next()?;
    if domain.is_empty() {
        None
    } else {
        Some(domain.to_lowercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn create_entry() {
        let entry = HistoryEntry::new("https://example.com", "Example");
        assert_eq!(entry.protocol, Protocol::Https);
        assert!(!entry.is_decentralized());
    }

    #[test]
    fn entry_with_duration() {
        let entry = HistoryEntry::new("ipfs://QmTest", "IPFS").with_duration(120);
        assert_eq!(entry.duration_secs, Some(120));
        assert!(entry.is_decentralized());
    }

    #[test]
    fn record_and_retrieve() {
        let mut history = BrowsingHistory::new(100);
        history.record(HistoryEntry::new("https://a.com", "A"));
        history.record(HistoryEntry::new("https://b.com", "B"));
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn max_entries_eviction() {
        let mut history = BrowsingHistory::new(3);
        for i in 0..5 {
            history.record(HistoryEntry::new(
                format!("https://{i}.com"),
                format!("Page {i}"),
            ));
        }
        assert_eq!(history.len(), 3);
        // Oldest entries evicted
        assert!(history.search("0").is_empty());
        assert!(history.search("1").is_empty());
        assert!(!history.search("4").is_empty());
    }

    #[test]
    fn search_by_title() {
        let mut history = BrowsingHistory::new(100);
        history.record(HistoryEntry::new(
            "https://rust-lang.org",
            "Rust Programming",
        ));
        history.record(HistoryEntry::new("https://go.dev", "Go Language"));

        assert_eq!(history.search("rust").len(), 1);
        assert_eq!(history.search("language").len(), 1);
        assert_eq!(history.search("python").len(), 0);
    }

    #[test]
    fn search_by_url() {
        let mut history = BrowsingHistory::new(100);
        history.record(HistoryEntry::new("https://example.com/page", "Example"));
        assert_eq!(history.search("example.com").len(), 1);
    }

    #[test]
    fn since_filter() {
        let mut history = BrowsingHistory::new(100);
        history.record(HistoryEntry::new("https://a.com", "A"));

        let future = Utc::now() + Duration::hours(1);
        assert!(history.since(future).is_empty());

        let past = Utc::now() - Duration::hours(1);
        assert_eq!(history.since(past).len(), 1);
    }

    #[test]
    fn by_domain() {
        let mut history = BrowsingHistory::new(100);
        history.record(HistoryEntry::new("https://github.com/user/repo", "Repo"));
        history.record(HistoryEntry::new("https://github.com/issues", "Issues"));
        history.record(HistoryEntry::new("https://google.com", "Google"));

        assert_eq!(history.by_domain("github.com").len(), 2);
        assert_eq!(history.by_domain("google.com").len(), 1);
    }

    #[test]
    fn decentralized_only() {
        let mut history = BrowsingHistory::new(100);
        history.record(HistoryEntry::new("https://google.com", "Google"));
        history.record(HistoryEntry::new("ipfs://QmTest", "IPFS Page"));
        history.record(HistoryEntry::new("ar://txid", "Arweave"));

        assert_eq!(history.decentralized_only().len(), 2);
    }

    #[test]
    fn recent() {
        let mut history = BrowsingHistory::new(100);
        for i in 0..10 {
            history.record(HistoryEntry::new(
                format!("https://{i}.com"),
                format!("{i}"),
            ));
        }

        let recent = history.recent(3);
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].title, "9");
    }

    #[test]
    fn clear() {
        let mut history = BrowsingHistory::new(100);
        history.record(HistoryEntry::new("https://a.com", "A"));
        history.clear();
        assert!(history.is_empty());
    }

    #[test]
    fn clear_domain() {
        let mut history = BrowsingHistory::new(100);
        history.record(HistoryEntry::new("https://github.com/a", "A"));
        history.record(HistoryEntry::new("https://github.com/b", "B"));
        history.record(HistoryEntry::new("https://google.com", "C"));

        history.clear_domain("github.com");
        assert_eq!(history.len(), 1);
        assert_eq!(history.search("google").len(), 1);
    }

    #[test]
    fn total_time() {
        let mut history = BrowsingHistory::new(100);
        history.record(HistoryEntry::new("a", "A").with_duration(60));
        history.record(HistoryEntry::new("b", "B").with_duration(120));
        history.record(HistoryEntry::new("c", "C")); // no duration

        assert_eq!(history.total_time_secs(), 180);
    }

    #[test]
    fn unique_domains() {
        let mut history = BrowsingHistory::new(100);
        history.record(HistoryEntry::new("https://a.com/page1", "A1"));
        history.record(HistoryEntry::new("https://a.com/page2", "A2"));
        history.record(HistoryEntry::new("https://b.com", "B"));
        history.record(HistoryEntry::new("ipfs://QmTest", "IPFS"));

        let domains = history.unique_domains();
        assert_eq!(domains.len(), 3);
    }

    #[test]
    fn extract_domain_works() {
        assert_eq!(
            extract_domain("https://example.com/path"),
            Some("example.com".to_string())
        );
        assert_eq!(
            extract_domain("ipfs://QmTest123"),
            Some("qmtest123".to_string())
        );
    }

    #[test]
    fn history_serializes() {
        let mut history = BrowsingHistory::new(100);
        history.record(HistoryEntry::new("https://a.com", "A"));
        let json = serde_json::to_string(&history).unwrap();
        let restored: BrowsingHistory = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.len(), 1);
    }
}
