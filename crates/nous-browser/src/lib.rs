pub mod bookmarks;
pub mod config;
pub mod content_filter;
pub mod gateway;
pub mod history;
pub mod identity_switch;
pub mod resolver;
pub mod tab;

pub use bookmarks::{Bookmark, BookmarkStore};
pub use config::BrowserConfig;
pub use content_filter::{ContentFilter, FilterAction, FilterRule, RuleCategory};
pub use gateway::{FetchedContent, Gateway, GatewayError, PublicIpfsGateway, StubEnsResolver};
pub use history::{BrowsingHistory, HistoryEntry};
pub use identity_switch::IdentityRouter;
pub use resolver::{Protocol, ResolvedUrl, UrlResolver};
pub use tab::{Tab, TabManager, TabStatus};
