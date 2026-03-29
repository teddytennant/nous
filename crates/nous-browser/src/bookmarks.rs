use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    pub id: String,
    pub url: String,
    pub title: String,
    pub folder: Option<String>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub visit_count: u32,
}

impl Bookmark {
    pub fn new(url: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            url: url.into(),
            title: title.into(),
            folder: None,
            tags: Vec::new(),
            created_at: Utc::now(),
            visit_count: 0,
        }
    }

    pub fn in_folder(mut self, folder: impl Into<String>) -> Self {
        self.folder = Some(folder.into());
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn visit(&mut self) {
        self.visit_count += 1;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkStore {
    bookmarks: Vec<Bookmark>,
    folders: Vec<String>,
}

impl BookmarkStore {
    pub fn new() -> Self {
        Self {
            bookmarks: Vec::new(),
            folders: Vec::new(),
        }
    }

    pub fn add(&mut self, bookmark: Bookmark) {
        if let Some(ref folder) = bookmark.folder
            && !self.folders.contains(folder)
        {
            self.folders.push(folder.clone());
        }
        self.bookmarks.push(bookmark);
    }

    pub fn remove(&mut self, id: &str) -> bool {
        let before = self.bookmarks.len();
        self.bookmarks.retain(|b| b.id != id);
        self.bookmarks.len() < before
    }

    pub fn find_by_url(&self, url: &str) -> Option<&Bookmark> {
        self.bookmarks.iter().find(|b| b.url == url)
    }

    pub fn in_folder(&self, folder: &str) -> Vec<&Bookmark> {
        self.bookmarks
            .iter()
            .filter(|b| b.folder.as_deref() == Some(folder))
            .collect()
    }

    pub fn search(&self, query: &str) -> Vec<&Bookmark> {
        let q = query.to_lowercase();
        self.bookmarks
            .iter()
            .filter(|b| {
                b.title.to_lowercase().contains(&q)
                    || b.url.to_lowercase().contains(&q)
                    || b.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .collect()
    }

    pub fn most_visited(&self, limit: usize) -> Vec<&Bookmark> {
        let mut sorted: Vec<&Bookmark> = self.bookmarks.iter().collect();
        sorted.sort_by(|a, b| b.visit_count.cmp(&a.visit_count));
        sorted.truncate(limit);
        sorted
    }

    pub fn len(&self) -> usize {
        self.bookmarks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bookmarks.is_empty()
    }

    pub fn folders(&self) -> &[String] {
        &self.folders
    }

    /// Merge with another BookmarkStore (CRDT-like: union of bookmarks by id)
    pub fn merge(&mut self, other: &BookmarkStore) {
        let existing_ids: HashMap<String, usize> = self
            .bookmarks
            .iter()
            .enumerate()
            .map(|(i, b)| (b.id.clone(), i))
            .collect();

        let mut to_add = Vec::new();
        let mut to_update = Vec::new();

        for bookmark in &other.bookmarks {
            if let Some(&idx) = existing_ids.get(&bookmark.id) {
                if bookmark.visit_count > self.bookmarks[idx].visit_count {
                    to_update.push((idx, bookmark.clone()));
                }
            } else {
                to_add.push(bookmark.clone());
            }
        }

        for (idx, bm) in to_update {
            self.bookmarks[idx] = bm;
        }
        for bm in to_add {
            self.add(bm);
        }
    }
}

impl Default for BookmarkStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_bookmark() {
        let bm = Bookmark::new("ipfs://QmTest", "Test Page");
        assert_eq!(bm.url, "ipfs://QmTest");
        assert_eq!(bm.visit_count, 0);
    }

    #[test]
    fn bookmark_with_folder_and_tags() {
        let bm = Bookmark::new("https://example.com", "Example")
            .in_folder("Work")
            .with_tag("reference");
        assert_eq!(bm.folder.as_deref(), Some("Work"));
        assert_eq!(bm.tags, vec!["reference"]);
    }

    #[test]
    fn bookmark_visit() {
        let mut bm = Bookmark::new("test", "test");
        bm.visit();
        bm.visit();
        assert_eq!(bm.visit_count, 2);
    }

    #[test]
    fn store_add_remove() {
        let mut store = BookmarkStore::new();
        let bm = Bookmark::new("test", "Test");
        let id = bm.id.clone();
        store.add(bm);
        assert_eq!(store.len(), 1);

        assert!(store.remove(&id));
        assert!(store.is_empty());
    }

    #[test]
    fn store_find_by_url() {
        let mut store = BookmarkStore::new();
        store.add(Bookmark::new("https://a.com", "A"));
        store.add(Bookmark::new("https://b.com", "B"));

        assert!(store.find_by_url("https://a.com").is_some());
        assert!(store.find_by_url("https://c.com").is_none());
    }

    #[test]
    fn store_folders() {
        let mut store = BookmarkStore::new();
        store.add(Bookmark::new("a", "A").in_folder("Work"));
        store.add(Bookmark::new("b", "B").in_folder("Work"));
        store.add(Bookmark::new("c", "C").in_folder("Personal"));

        assert_eq!(store.in_folder("Work").len(), 2);
        assert_eq!(store.folders().len(), 2);
    }

    #[test]
    fn store_search() {
        let mut store = BookmarkStore::new();
        store.add(Bookmark::new("https://rust-lang.org", "Rust Language").with_tag("dev"));
        store.add(Bookmark::new("https://news.ycombinator.com", "Hacker News"));

        assert_eq!(store.search("rust").len(), 1);
        assert_eq!(store.search("dev").len(), 1);
        assert_eq!(store.search("news").len(), 1);
    }

    #[test]
    fn store_most_visited() {
        let mut store = BookmarkStore::new();
        let mut bm1 = Bookmark::new("a", "A");
        bm1.visit_count = 10;
        let mut bm2 = Bookmark::new("b", "B");
        bm2.visit_count = 5;
        let mut bm3 = Bookmark::new("c", "C");
        bm3.visit_count = 20;

        store.add(bm1);
        store.add(bm2);
        store.add(bm3);

        let top = store.most_visited(2);
        assert_eq!(top[0].title, "C");
        assert_eq!(top[1].title, "A");
    }

    #[test]
    fn store_merge() {
        let mut store1 = BookmarkStore::new();
        let bm = Bookmark::new("a", "A");
        let id = bm.id.clone();
        store1.add(bm);

        let mut store2 = BookmarkStore::new();
        store2.add(Bookmark::new("b", "B"));
        let mut bm_same = Bookmark::new("a", "A Updated");
        bm_same.id = id;
        bm_same.visit_count = 5;
        store2.add(bm_same);

        store1.merge(&store2);
        assert_eq!(store1.len(), 2);
        // The merged one should have higher visit count
        assert!(store1.bookmarks.iter().any(|b| b.visit_count == 5));
    }

    #[test]
    fn store_serializes() {
        let mut store = BookmarkStore::new();
        store.add(Bookmark::new("test", "Test").in_folder("Dev"));
        let json = serde_json::to_string(&store).unwrap();
        let restored: BookmarkStore = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.len(), 1);
    }
}
