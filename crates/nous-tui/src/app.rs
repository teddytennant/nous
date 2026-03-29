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
    pub connected: bool,
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
    fn app_defaults_disconnected() {
        let app = App::new(TuiConfig::default());
        assert!(!app.connected);
        assert!(app.api_client.is_none());
        assert!(app.balances.is_empty());
        assert!(app.daos.is_empty());
    }

    #[test]
    fn set_api_client() {
        let mut app = App::new(TuiConfig::default());
        app.set_api_client(crate::client::ApiClient::new("http://localhost:8080/api/v1"));
        assert!(app.api_client.is_some());
    }
}
