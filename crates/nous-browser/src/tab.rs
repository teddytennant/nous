use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use nous_core::Error;

use crate::resolver::Protocol;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationEntry {
    pub url: String,
    pub title: String,
    pub protocol: Protocol,
    pub visited_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TabStatus {
    Loading,
    Ready,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tab {
    pub id: String,
    pub url: String,
    pub title: String,
    pub status: TabStatus,
    pub protocol: Protocol,
    pub identity_did: Option<String>,
    pub created_at: DateTime<Utc>,
    history: Vec<NavigationEntry>,
    history_index: usize,
    pub pinned: bool,
}

impl Tab {
    pub fn new(url: impl Into<String>, title: impl Into<String>) -> Self {
        let url = url.into();
        let protocol = Protocol::from_url(&url);
        let title = title.into();
        let now = Utc::now();

        let entry = NavigationEntry {
            url: url.clone(),
            title: title.clone(),
            protocol,
            visited_at: now,
        };

        Self {
            id: format!("tab:{}", Uuid::new_v4()),
            url,
            title,
            status: TabStatus::Ready,
            protocol,
            identity_did: None,
            created_at: now,
            history: vec![entry],
            history_index: 0,
            pinned: false,
        }
    }

    pub fn navigate(&mut self, url: impl Into<String>, title: impl Into<String>) {
        let url = url.into();
        let title = title.into();
        let protocol = Protocol::from_url(&url);

        // Truncate forward history
        self.history.truncate(self.history_index + 1);

        let entry = NavigationEntry {
            url: url.clone(),
            title: title.clone(),
            protocol,
            visited_at: Utc::now(),
        };

        self.history.push(entry);
        self.history_index = self.history.len() - 1;
        self.url = url;
        self.title = title;
        self.protocol = protocol;
        self.status = TabStatus::Ready;
    }

    pub fn go_back(&mut self) -> Result<(), Error> {
        if !self.can_go_back() {
            return Err(Error::InvalidInput("no previous page".into()));
        }
        self.history_index -= 1;
        let entry = &self.history[self.history_index];
        self.url = entry.url.clone();
        self.title = entry.title.clone();
        self.protocol = entry.protocol;
        Ok(())
    }

    pub fn go_forward(&mut self) -> Result<(), Error> {
        if !self.can_go_forward() {
            return Err(Error::InvalidInput("no next page".into()));
        }
        self.history_index += 1;
        let entry = &self.history[self.history_index];
        self.url = entry.url.clone();
        self.title = entry.title.clone();
        self.protocol = entry.protocol;
        Ok(())
    }

    pub fn can_go_back(&self) -> bool {
        self.history_index > 0
    }

    pub fn can_go_forward(&self) -> bool {
        self.history_index < self.history.len() - 1
    }

    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    pub fn pin(&mut self) {
        self.pinned = true;
    }

    pub fn unpin(&mut self) {
        self.pinned = false;
    }

    pub fn set_identity(&mut self, did: impl Into<String>) {
        self.identity_did = Some(did.into());
    }

    pub fn set_error(&mut self) {
        self.status = TabStatus::Error;
    }

    pub fn set_loading(&mut self) {
        self.status = TabStatus::Loading;
    }

    pub fn is_decentralized(&self) -> bool {
        self.protocol.is_decentralized()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabManager {
    tabs: Vec<Tab>,
    active_index: usize,
}

impl TabManager {
    pub fn new() -> Self {
        let initial = Tab::new("nous://home", "New Tab");
        Self {
            tabs: vec![initial],
            active_index: 0,
        }
    }

    pub fn open(&mut self, url: impl Into<String>, title: impl Into<String>) -> &Tab {
        let tab = Tab::new(url, title);
        self.tabs.push(tab);
        self.active_index = self.tabs.len() - 1;
        &self.tabs[self.active_index]
    }

    pub fn close(&mut self, tab_id: &str) -> Result<(), Error> {
        if self.tabs.len() <= 1 {
            return Err(Error::InvalidInput("cannot close last tab".into()));
        }

        let idx = self
            .tabs
            .iter()
            .position(|t| t.id == tab_id)
            .ok_or_else(|| Error::NotFound(format!("tab {tab_id} not found")))?;

        if self.tabs[idx].pinned {
            return Err(Error::InvalidInput("cannot close pinned tab".into()));
        }

        self.tabs.remove(idx);
        if self.active_index >= self.tabs.len() {
            self.active_index = self.tabs.len() - 1;
        } else if self.active_index > idx {
            self.active_index -= 1;
        }

        Ok(())
    }

    pub fn activate(&mut self, tab_id: &str) -> Result<(), Error> {
        let idx = self
            .tabs
            .iter()
            .position(|t| t.id == tab_id)
            .ok_or_else(|| Error::NotFound(format!("tab {tab_id} not found")))?;
        self.active_index = idx;
        Ok(())
    }

    pub fn active(&self) -> &Tab {
        &self.tabs[self.active_index]
    }

    pub fn active_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active_index]
    }

    pub fn tab(&self, tab_id: &str) -> Option<&Tab> {
        self.tabs.iter().find(|t| t.id == tab_id)
    }

    pub fn tab_mut(&mut self, tab_id: &str) -> Option<&mut Tab> {
        self.tabs.iter_mut().find(|t| t.id == tab_id)
    }

    pub fn tabs(&self) -> &[Tab] {
        &self.tabs
    }

    pub fn count(&self) -> usize {
        self.tabs.len()
    }

    pub fn active_index(&self) -> usize {
        self.active_index
    }

    pub fn move_tab(&mut self, from: usize, to: usize) -> Result<(), Error> {
        if from >= self.tabs.len() || to >= self.tabs.len() {
            return Err(Error::InvalidInput("index out of bounds".into()));
        }
        let tab = self.tabs.remove(from);
        self.tabs.insert(to, tab);

        // Adjust active index
        if self.active_index == from {
            self.active_index = to;
        } else if from < self.active_index && to >= self.active_index {
            self.active_index -= 1;
        } else if from > self.active_index && to <= self.active_index {
            self.active_index += 1;
        }

        Ok(())
    }
}

impl Default for TabManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_tab() {
        let tab = Tab::new("https://example.com", "Example");
        assert!(tab.id.starts_with("tab:"));
        assert_eq!(tab.status, TabStatus::Ready);
        assert_eq!(tab.protocol, Protocol::Https);
        assert!(!tab.pinned);
    }

    #[test]
    fn tab_navigation() {
        let mut tab = Tab::new("https://a.com", "A");
        assert!(!tab.can_go_back());

        tab.navigate("https://b.com", "B");
        assert!(tab.can_go_back());
        assert!(!tab.can_go_forward());
        assert_eq!(tab.url, "https://b.com");
        assert_eq!(tab.history_len(), 2);

        tab.go_back().unwrap();
        assert_eq!(tab.url, "https://a.com");
        assert!(tab.can_go_forward());

        tab.go_forward().unwrap();
        assert_eq!(tab.url, "https://b.com");
    }

    #[test]
    fn navigation_truncates_forward_history() {
        let mut tab = Tab::new("https://a.com", "A");
        tab.navigate("https://b.com", "B");
        tab.navigate("https://c.com", "C");
        tab.go_back().unwrap();
        tab.go_back().unwrap();

        // Navigate from A, should truncate B and C
        tab.navigate("https://d.com", "D");
        assert!(!tab.can_go_forward());
        assert_eq!(tab.history_len(), 2); // A, D
    }

    #[test]
    fn cannot_go_back_at_start() {
        let mut tab = Tab::new("https://a.com", "A");
        assert!(tab.go_back().is_err());
    }

    #[test]
    fn cannot_go_forward_at_end() {
        let mut tab = Tab::new("https://a.com", "A");
        assert!(tab.go_forward().is_err());
    }

    #[test]
    fn tab_pin_unpin() {
        let mut tab = Tab::new("https://a.com", "A");
        tab.pin();
        assert!(tab.pinned);
        tab.unpin();
        assert!(!tab.pinned);
    }

    #[test]
    fn tab_identity() {
        let mut tab = Tab::new("https://a.com", "A");
        tab.set_identity("did:key:anon");
        assert_eq!(tab.identity_did.as_deref(), Some("did:key:anon"));
    }

    #[test]
    fn tab_status_transitions() {
        let mut tab = Tab::new("https://a.com", "A");
        tab.set_loading();
        assert_eq!(tab.status, TabStatus::Loading);
        tab.set_error();
        assert_eq!(tab.status, TabStatus::Error);
    }

    #[test]
    fn ipfs_tab_is_decentralized() {
        let tab = Tab::new("ipfs://QmTest", "IPFS Page");
        assert!(tab.is_decentralized());
    }

    #[test]
    fn https_tab_is_not_decentralized() {
        let tab = Tab::new("https://google.com", "Google");
        assert!(!tab.is_decentralized());
    }

    // ── TabManager Tests ──────────────────────────────────────────────

    #[test]
    fn manager_starts_with_one_tab() {
        let mgr = TabManager::new();
        assert_eq!(mgr.count(), 1);
        assert_eq!(mgr.active().url, "nous://home");
    }

    #[test]
    fn open_and_activate() {
        let mut mgr = TabManager::new();
        let tab = mgr.open("https://a.com", "A");
        let id = tab.id.clone();

        assert_eq!(mgr.count(), 2);
        assert_eq!(mgr.active().url, "https://a.com");

        // Switch back to first tab
        let first_id = mgr.tabs()[0].id.clone();
        mgr.activate(&first_id).unwrap();
        assert_eq!(mgr.active().url, "nous://home");

        // Switch to second tab
        mgr.activate(&id).unwrap();
        assert_eq!(mgr.active().url, "https://a.com");
    }

    #[test]
    fn close_tab() {
        let mut mgr = TabManager::new();
        let tab = mgr.open("https://a.com", "A");
        let id = tab.id.clone();

        mgr.close(&id).unwrap();
        assert_eq!(mgr.count(), 1);
    }

    #[test]
    fn cannot_close_last_tab() {
        let mut mgr = TabManager::new();
        let id = mgr.active().id.clone();
        assert!(mgr.close(&id).is_err());
    }

    #[test]
    fn cannot_close_pinned_tab() {
        let mut mgr = TabManager::new();
        let tab = mgr.open("https://a.com", "A");
        let id = tab.id.clone();
        mgr.tab_mut(&id).unwrap().pin();
        assert!(mgr.close(&id).is_err());
    }

    #[test]
    fn close_adjusts_active_index() {
        let mut mgr = TabManager::new();
        let _tab1 = mgr.open("https://a.com", "A");
        let tab2 = mgr.open("https://b.com", "B");
        let id2 = tab2.id.clone();

        // Active is tab2 (index 2). Close it.
        mgr.close(&id2).unwrap();
        // Active should now be the last tab
        assert_eq!(mgr.active_index(), 1);
    }

    #[test]
    fn activate_nonexistent() {
        let mut mgr = TabManager::new();
        assert!(mgr.activate("nonexistent").is_err());
    }

    #[test]
    fn navigate_active_tab() {
        let mut mgr = TabManager::new();
        mgr.active_mut().navigate("https://a.com", "A");
        assert_eq!(mgr.active().url, "https://a.com");
    }

    #[test]
    fn move_tab() {
        let mut mgr = TabManager::new();
        mgr.open("https://a.com", "A");
        mgr.open("https://b.com", "B");

        // Move tab at index 0 to index 2
        mgr.move_tab(0, 2).unwrap();
        assert_eq!(mgr.tabs()[2].url, "nous://home");
    }

    #[test]
    fn move_tab_out_of_bounds() {
        let mut mgr = TabManager::new();
        assert!(mgr.move_tab(0, 5).is_err());
    }

    #[test]
    fn tab_manager_serializes() {
        let mut mgr = TabManager::new();
        mgr.open("https://a.com", "A");
        let json = serde_json::to_string(&mgr).unwrap();
        let restored: TabManager = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.count(), 2);
    }
}
