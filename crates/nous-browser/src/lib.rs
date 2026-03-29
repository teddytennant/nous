pub mod bookmarks;
pub mod config;
pub mod identity_switch;
pub mod resolver;

pub use bookmarks::{Bookmark, BookmarkStore};
pub use config::BrowserConfig;
pub use identity_switch::IdentityRouter;
pub use resolver::{Protocol, ResolvedUrl, UrlResolver};
