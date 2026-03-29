use crate::config::TuiConfig;
use crate::input::InputField;
use crate::tabs::{Tab, TabState};

pub struct App {
    pub config: TuiConfig,
    pub tabs: TabState,
    pub input: InputField,
    pub running: bool,
    pub peer_count: usize,
    pub local_did: String,
    pub messages: Vec<DisplayMessage>,
    pub feed_items: Vec<FeedItem>,
    pub scroll_offset: usize,
    pub api_client: Option<crate::client::ApiClient>,
    pub node_status: Option<String>,
    pub node_version: Option<String>,
    pub node_uptime: Option<u64>,
    pub balances: Vec<crate::client::BalanceEntry>,
    pub daos: Vec<crate::client::DaoItem>,
    pub proposals: Vec<crate::client::ProposalItem>,
    pub channels: Vec<crate::client::ChannelListItem>,
    pub listings: Vec<crate::client::ListingItem>,
    pub orders: Vec<crate::client::OrderItem>,
    pub marketplace_tab: MarketplaceSubTab,
    pub marketplace_selected: usize,
    pub browser_urls: Vec<BrowserTabEntry>,
    pub browser_selected: usize,
    pub browser_history_count: usize,
    pub browser_blocked_count: u64,
    pub browser_filter_rules: usize,
    pub connected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketplaceSubTab {
    Listings,
    Orders,
}

#[derive(Debug, Clone)]
pub struct BrowserTabEntry {
    pub title: String,
    pub url: String,
    pub status: String,
    pub pinned: bool,
}

#[derive(Debug, Clone)]
pub struct DisplayMessage {
    pub sender: String,
    pub content: String,
    pub timestamp: String,
}

#[derive(Debug, Clone)]
pub struct FeedItem {
    pub author: String,
    pub content: String,
    pub timestamp: String,
    pub reactions: u32,
    pub replies: u32,
}

impl App {
    pub fn new(config: TuiConfig) -> Self {
        Self {
            config,
            tabs: TabState::new(),
            input: InputField::new("Type a message..."),
            running: true,
            peer_count: 0,
            local_did: String::new(),
            messages: Vec::new(),
            feed_items: Vec::new(),
            scroll_offset: 0,
            api_client: None,
            node_status: None,
            node_version: None,
            node_uptime: None,
            balances: Vec::new(),
            daos: Vec::new(),
            proposals: Vec::new(),
            channels: Vec::new(),
            listings: Vec::new(),
            orders: Vec::new(),
            marketplace_tab: MarketplaceSubTab::Listings,
            marketplace_selected: 0,
            browser_urls: Vec::new(),
            browser_selected: 0,
            browser_history_count: 0,
            browser_blocked_count: 0,
            browser_filter_rules: 0,
            connected: false,
        }
    }

    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn set_api_client(&mut self, client: crate::client::ApiClient) {
        self.api_client = Some(client);
    }

    pub fn handle_key(&mut self, c: char) {
        if let Some(tab) = Tab::from_shortcut(c) {
            self.tabs.select(tab);
        } else {
            self.input.insert(c);
        }
    }

    pub fn submit_input(&mut self) -> Option<String> {
        if self.input.is_empty() {
            return None;
        }
        Some(self.input.take())
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    pub fn scroll_down(&mut self, max: usize) {
        if self.scroll_offset < max {
            self.scroll_offset += 1;
        }
    }

    pub fn add_message(&mut self, msg: DisplayMessage) {
        self.messages.push(msg);
        if self.messages.len() > self.config.max_visible_messages {
            self.messages.remove(0);
        }
    }

    pub fn marketplace_list_len(&self) -> usize {
        match self.marketplace_tab {
            MarketplaceSubTab::Listings => self.listings.len(),
            MarketplaceSubTab::Orders => self.orders.len(),
        }
    }

    pub fn marketplace_select_up(&mut self) {
        self.marketplace_selected = self.marketplace_selected.saturating_sub(1);
    }

    pub fn marketplace_select_down(&mut self) {
        let max = self.marketplace_list_len().saturating_sub(1);
        if self.marketplace_selected < max {
            self.marketplace_selected += 1;
        }
    }

    pub fn marketplace_toggle_tab(&mut self) {
        self.marketplace_tab = match self.marketplace_tab {
            MarketplaceSubTab::Listings => MarketplaceSubTab::Orders,
            MarketplaceSubTab::Orders => MarketplaceSubTab::Listings,
        };
        self.marketplace_selected = 0;
    }

    pub fn browser_select_up(&mut self) {
        self.browser_selected = self.browser_selected.saturating_sub(1);
    }

    pub fn browser_select_down(&mut self) {
        let max = self.browser_urls.len().saturating_sub(1);
        if self.browser_selected < max {
            self.browser_selected += 1;
        }
    }

    pub fn add_feed_item(&mut self, item: FeedItem) {
        self.feed_items.insert(0, item);
    }

    pub fn visible_messages(&self) -> &[DisplayMessage] {
        let end = self.messages.len();
        let start = end.saturating_sub(self.config.max_visible_messages);
        &self.messages[start..end]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_creates() {
        let app = App::new(TuiConfig::default());
        assert!(app.running);
        assert_eq!(app.tabs.active, Tab::Feed);
    }

    #[test]
    fn quit() {
        let mut app = App::new(TuiConfig::default());
        app.quit();
        assert!(!app.running);
    }

    #[test]
    fn handle_tab_shortcuts() {
        let mut app = App::new(TuiConfig::default());
        app.handle_key('3');
        assert_eq!(app.tabs.active, Tab::Governance);
    }

    #[test]
    fn handle_text_input() {
        let mut app = App::new(TuiConfig::default());
        app.handle_key('h');
        // 'h' is not a tab shortcut, so it goes to input... wait, no shortcuts are 'h'
        // Actually '1'-'7' are shortcuts. 'h' should be text input.
        // But handle_key checks shortcuts first. Let me verify behavior.
        // Tab::from_shortcut('h') returns None, so it falls through to input.insert
        assert_eq!(app.input.value, "h");
    }

    #[test]
    fn submit_input() {
        let mut app = App::new(TuiConfig::default());
        app.input.insert('h');
        app.input.insert('i');
        let val = app.submit_input();
        assert_eq!(val, Some("hi".to_string()));
        assert!(app.input.is_empty());
    }

    #[test]
    fn submit_empty_input() {
        let mut app = App::new(TuiConfig::default());
        assert_eq!(app.submit_input(), None);
    }

    #[test]
    fn scroll() {
        let mut app = App::new(TuiConfig::default());
        app.scroll_down(10);
        assert_eq!(app.scroll_offset, 1);
        app.scroll_up();
        assert_eq!(app.scroll_offset, 0);
        app.scroll_up(); // shouldn't go below 0
        assert_eq!(app.scroll_offset, 0);
    }

    #[test]
    fn add_message() {
        let mut app = App::new(TuiConfig::default());
        app.add_message(DisplayMessage {
            sender: "alice".into(),
            content: "hello".into(),
            timestamp: "12:00".into(),
        });
        assert_eq!(app.messages.len(), 1);
    }

    #[test]
    fn message_limit() {
        let mut config = TuiConfig::default();
        config.max_visible_messages = 3;
        let mut app = App::new(config);

        for i in 0..5 {
            app.add_message(DisplayMessage {
                sender: "alice".into(),
                content: format!("msg {i}"),
                timestamp: "12:00".into(),
            });
        }
        assert_eq!(app.messages.len(), 3);
    }

    #[test]
    fn feed_items_newest_first() {
        let mut app = App::new(TuiConfig::default());
        app.add_feed_item(FeedItem {
            author: "alice".into(),
            content: "first".into(),
            timestamp: "12:00".into(),
            reactions: 0,
            replies: 0,
        });
        app.add_feed_item(FeedItem {
            author: "bob".into(),
            content: "second".into(),
            timestamp: "12:01".into(),
            reactions: 0,
            replies: 0,
        });
        assert_eq!(app.feed_items[0].content, "second");
    }

    #[test]
    fn marketplace_navigation() {
        let mut app = App::new(TuiConfig::default());
        app.listings.push(crate::client::ListingItem {
            id: "l1".into(),
            seller_did: "d".into(),
            title: "A".into(),
            description: "".into(),
            category: "Physical".into(),
            price_token: "ETH".into(),
            price_amount: "1".into(),
            status: "Active".into(),
            created_at: "".into(),
            tags: vec![],
        });
        app.listings.push(crate::client::ListingItem {
            id: "l2".into(),
            seller_did: "d".into(),
            title: "B".into(),
            description: "".into(),
            category: "Digital".into(),
            price_token: "ETH".into(),
            price_amount: "2".into(),
            status: "Active".into(),
            created_at: "".into(),
            tags: vec![],
        });

        assert_eq!(app.marketplace_selected, 0);
        app.marketplace_select_down();
        assert_eq!(app.marketplace_selected, 1);
        app.marketplace_select_down(); // at max
        assert_eq!(app.marketplace_selected, 1);
        app.marketplace_select_up();
        assert_eq!(app.marketplace_selected, 0);
        app.marketplace_select_up(); // at min
        assert_eq!(app.marketplace_selected, 0);
    }

    #[test]
    fn marketplace_toggle_tab() {
        let mut app = App::new(TuiConfig::default());
        assert_eq!(app.marketplace_tab, MarketplaceSubTab::Listings);
        app.marketplace_selected = 5;
        app.marketplace_toggle_tab();
        assert_eq!(app.marketplace_tab, MarketplaceSubTab::Orders);
        assert_eq!(app.marketplace_selected, 0); // reset on toggle
        app.marketplace_toggle_tab();
        assert_eq!(app.marketplace_tab, MarketplaceSubTab::Listings);
    }

    #[test]
    fn browser_navigation() {
        let mut app = App::new(TuiConfig::default());
        app.browser_urls.push(BrowserTabEntry {
            title: "A".into(),
            url: "a".into(),
            status: "Ready".into(),
            pinned: false,
        });
        app.browser_urls.push(BrowserTabEntry {
            title: "B".into(),
            url: "b".into(),
            status: "Ready".into(),
            pinned: false,
        });

        assert_eq!(app.browser_selected, 0);
        app.browser_select_down();
        assert_eq!(app.browser_selected, 1);
        app.browser_select_down();
        assert_eq!(app.browser_selected, 1);
        app.browser_select_up();
        assert_eq!(app.browser_selected, 0);
    }

    #[test]
    fn app_defaults_disconnected() {
        let app = App::new(TuiConfig::default());
        assert!(!app.connected);
        assert!(app.api_client.is_none());
        assert!(app.balances.is_empty());
        assert!(app.daos.is_empty());
        assert!(app.listings.is_empty());
        assert!(app.orders.is_empty());
        assert_eq!(app.marketplace_tab, MarketplaceSubTab::Listings);
        assert!(app.browser_urls.is_empty());
    }

    #[test]
    fn set_api_client() {
        let mut app = App::new(TuiConfig::default());
        app.set_api_client(crate::client::ApiClient::new(
            "http://localhost:8080/api/v1",
        ));
        assert!(app.api_client.is_some());
    }
}
