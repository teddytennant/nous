use nous_core::Result;

use crate::sqlite::Database;

pub struct KvStore<'a> {
    db: &'a Database,
    namespace: String,
}

impl<'a> KvStore<'a> {
    pub fn new(db: &'a Database, namespace: impl Into<String>) -> Self {
        Self {
            db,
            namespace: namespace.into(),
        }
    }

    fn prefixed_key(&self, key: &str) -> String {
        format!("{}:{}", self.namespace, key)
    }

    pub fn put(&self, key: &str, value: &[u8]) -> Result<()> {
        self.db.put_kv(&self.prefixed_key(key), value)
    }

    pub fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        self.db.get_kv(&self.prefixed_key(key))
    }

    pub fn delete(&self, key: &str) -> Result<bool> {
        self.db.delete_kv(&self.prefixed_key(key))
    }

    pub fn put_json<T: serde::Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let bytes = serde_json::to_vec(value)?;
        self.put(key, &bytes)
    }

    pub fn get_json<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        match self.get(key)? {
            Some(bytes) => {
                let value = serde_json::from_slice(&bytes)?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn namespaced_keys() {
        let db = Database::in_memory().unwrap();
        let store_a = KvStore::new(&db, "app-a");
        let store_b = KvStore::new(&db, "app-b");

        store_a.put("key", b"value-a").unwrap();
        store_b.put("key", b"value-b").unwrap();

        assert_eq!(store_a.get("key").unwrap().unwrap(), b"value-a");
        assert_eq!(store_b.get("key").unwrap().unwrap(), b"value-b");
    }

    #[test]
    fn json_roundtrip() {
        let db = Database::in_memory().unwrap();
        let store = KvStore::new(&db, "test");

        let data = serde_json::json!({"name": "nous", "version": 1});
        store.put_json("config", &data).unwrap();

        let retrieved: serde_json::Value = store.get_json("config").unwrap().unwrap();
        assert_eq!(retrieved, data);
    }

    #[test]
    fn get_missing_json_returns_none() {
        let db = Database::in_memory().unwrap();
        let store = KvStore::new(&db, "test");

        let result: Option<serde_json::Value> = store.get_json("missing").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn delete_namespaced() {
        let db = Database::in_memory().unwrap();
        let store = KvStore::new(&db, "ns");

        store.put("key", b"value").unwrap();
        assert!(store.delete("key").unwrap());
        assert!(store.get("key").unwrap().is_none());
    }
}
