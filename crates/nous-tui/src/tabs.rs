use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tab {
    Feed,
    Messages,
    Governance,
    Wallet,
    Marketplace,
    Browser,
    Identity,
    Peers,
    Settings,
}

impl Tab {
    pub fn label(&self) -> &str {
        match self {
            Self::Feed => "Feed",
            Self::Messages => "Messages",
            Self::Governance => "Governance",
            Self::Wallet => "Wallet",
            Self::Marketplace => "Market",
            Self::Browser => "Browser",
            Self::Identity => "Identity",
            Self::Peers => "Peers",
            Self::Settings => "Settings",
        }
    }

    pub fn shortcut(&self) -> char {
        match self {
            Self::Feed => '1',
            Self::Messages => '2',
            Self::Governance => '3',
            Self::Wallet => '4',
            Self::Marketplace => '5',
            Self::Browser => '6',
            Self::Identity => '7',
            Self::Peers => '8',
            Self::Settings => '9',
        }
    }

    pub fn all() -> &'static [Tab] {
        &[
            Tab::Feed,
            Tab::Messages,
            Tab::Governance,
            Tab::Wallet,
            Tab::Marketplace,
            Tab::Browser,
            Tab::Identity,
            Tab::Peers,
            Tab::Settings,
        ]
    }

    pub fn from_shortcut(c: char) -> Option<Self> {
        match c {
            '1' => Some(Self::Feed),
            '2' => Some(Self::Messages),
            '3' => Some(Self::Governance),
            '4' => Some(Self::Wallet),
            '5' => Some(Self::Marketplace),
            '6' => Some(Self::Browser),
            '7' => Some(Self::Identity),
            '8' => Some(Self::Peers),
            '9' => Some(Self::Settings),
            _ => None,
        }
    }
}

pub struct TabState {
    pub active: Tab,
}

impl TabState {
    pub fn new() -> Self {
        Self { active: Tab::Feed }
    }

    pub fn select(&mut self, tab: Tab) {
        self.active = tab;
    }

    pub fn next(&mut self) {
        let tabs = Tab::all();
        let idx = tabs.iter().position(|t| *t == self.active).unwrap_or(0);
        self.active = tabs[(idx + 1) % tabs.len()];
    }

    pub fn prev(&mut self) {
        let tabs = Tab::all();
        let idx = tabs.iter().position(|t| *t == self.active).unwrap_or(0);
        self.active = tabs[(idx + tabs.len() - 1) % tabs.len()];
    }
}

impl Default for TabState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_labels() {
        assert_eq!(Tab::Feed.label(), "Feed");
        assert_eq!(Tab::Wallet.label(), "Wallet");
    }

    #[test]
    fn tab_shortcuts() {
        for tab in Tab::all() {
            let shortcut = tab.shortcut();
            let recovered = Tab::from_shortcut(shortcut);
            assert_eq!(recovered, Some(*tab));
        }
    }

    #[test]
    fn tab_state_navigation() {
        let mut state = TabState::new();
        assert_eq!(state.active, Tab::Feed);

        state.next();
        assert_eq!(state.active, Tab::Messages);

        state.next();
        assert_eq!(state.active, Tab::Governance);

        state.prev();
        assert_eq!(state.active, Tab::Messages);
    }

    #[test]
    fn tab_wraps_around() {
        let mut state = TabState::new();
        state.select(Tab::Settings);
        state.next();
        assert_eq!(state.active, Tab::Feed);
    }

    #[test]
    fn tab_wraps_around_backward() {
        let mut state = TabState::new();
        state.prev();
        assert_eq!(state.active, Tab::Settings);
    }

    #[test]
    fn all_tabs_count() {
        assert_eq!(Tab::all().len(), 9);
    }

    #[test]
    fn invalid_shortcut() {
        assert_eq!(Tab::from_shortcut('x'), None);
    }
}
