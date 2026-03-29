use rusqlite::{Connection, params};

use nous_core::{Error, Result};

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(path: &std::path::Path) -> Result<Self> {
        let conn = Connection::open(path)
            .map_err(|e| Error::Storage(format!("failed to open db: {e}")))?;

        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;",
        )
        .map_err(|e| Error::Storage(format!("failed to set pragmas: {e}")))?;

        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()
            .map_err(|e| Error::Storage(format!("failed to open in-memory db: {e}")))?;

        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> Result<()> {
        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS kv (
                    key   TEXT PRIMARY KEY,
                    value BLOB NOT NULL,
                    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
                );

                CREATE TABLE IF NOT EXISTS identities (
                    did        TEXT PRIMARY KEY,
                    document   TEXT NOT NULL,
                    signing_key BLOB,
                    created_at TEXT NOT NULL DEFAULT (datetime('now'))
                );

                CREATE TABLE IF NOT EXISTS messages (
                    id         TEXT PRIMARY KEY,
                    channel_id TEXT NOT NULL,
                    sender_did TEXT NOT NULL,
                    payload    BLOB NOT NULL,
                    timestamp  TEXT NOT NULL,
                    FOREIGN KEY (sender_did) REFERENCES identities(did)
                );
                CREATE INDEX IF NOT EXISTS idx_messages_channel ON messages(channel_id, timestamp);

                CREATE TABLE IF NOT EXISTS credentials (
                    id          TEXT PRIMARY KEY,
                    issuer_did  TEXT NOT NULL,
                    subject_did TEXT NOT NULL,
                    credential  TEXT NOT NULL,
                    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
                );
                CREATE INDEX IF NOT EXISTS idx_credentials_subject ON credentials(subject_did);

                CREATE TABLE IF NOT EXISTS reputation_events (
                    id          TEXT PRIMARY KEY,
                    subject_did TEXT NOT NULL,
                    issuer_did  TEXT NOT NULL,
                    category    TEXT NOT NULL,
                    delta       INTEGER NOT NULL,
                    reason      TEXT NOT NULL,
                    timestamp   TEXT NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_reputation_subject ON reputation_events(subject_did);",
            )
            .map_err(|e| Error::Storage(format!("migration failed: {e}")))?;

        Ok(())
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn put_kv(&self, key: &str, value: &[u8]) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO kv (key, value, updated_at) VALUES (?1, ?2, datetime('now'))
                 ON CONFLICT(key) DO UPDATE SET value = ?2, updated_at = datetime('now')",
                params![key, value],
            )
            .map_err(|e| Error::Storage(format!("kv put failed: {e}")))?;
        Ok(())
    }

    pub fn get_kv(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM kv WHERE key = ?1")
            .map_err(|e| Error::Storage(format!("kv prepare failed: {e}")))?;

        let result = stmt
            .query_row(params![key], |row| row.get(0))
            .optional()
            .map_err(|e| Error::Storage(format!("kv get failed: {e}")))?;

        Ok(result)
    }

    pub fn delete_kv(&self, key: &str) -> Result<bool> {
        let count = self
            .conn
            .execute("DELETE FROM kv WHERE key = ?1", params![key])
            .map_err(|e| Error::Storage(format!("kv delete failed: {e}")))?;
        Ok(count > 0)
    }

    pub fn store_identity(
        &self,
        did: &str,
        document_json: &str,
        signing_key: Option<&[u8]>,
    ) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO identities (did, document, signing_key) VALUES (?1, ?2, ?3)
                 ON CONFLICT(did) DO UPDATE SET document = ?2, signing_key = ?3",
                params![did, document_json, signing_key],
            )
            .map_err(|e| Error::Storage(format!("store identity failed: {e}")))?;
        Ok(())
    }

    pub fn get_identity(&self, did: &str) -> Result<Option<(String, Option<Vec<u8>>)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT document, signing_key FROM identities WHERE did = ?1")
            .map_err(|e| Error::Storage(format!("prepare failed: {e}")))?;

        let result = stmt
            .query_row(params![did], |row| Ok((row.get(0)?, row.get(1)?)))
            .optional()
            .map_err(|e| Error::Storage(format!("get identity failed: {e}")))?;

        Ok(result)
    }
}

trait OptionalExt<T> {
    fn optional(self) -> std::result::Result<Option<T>, rusqlite::Error>;
}

impl<T> OptionalExt<T> for std::result::Result<T, rusqlite::Error> {
    fn optional(self) -> std::result::Result<Option<T>, rusqlite::Error> {
        match self {
            Ok(val) => Ok(Some(val)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_in_memory_database() {
        let db = Database::in_memory().unwrap();
        assert!(db.conn().is_autocommit());
    }

    #[test]
    fn kv_put_and_get() {
        let db = Database::in_memory().unwrap();
        db.put_kv("test-key", b"test-value").unwrap();

        let value = db.get_kv("test-key").unwrap().unwrap();
        assert_eq!(value, b"test-value");
    }

    #[test]
    fn kv_get_missing_returns_none() {
        let db = Database::in_memory().unwrap();
        assert!(db.get_kv("nonexistent").unwrap().is_none());
    }

    #[test]
    fn kv_upsert() {
        let db = Database::in_memory().unwrap();
        db.put_kv("key", b"v1").unwrap();
        db.put_kv("key", b"v2").unwrap();

        let value = db.get_kv("key").unwrap().unwrap();
        assert_eq!(value, b"v2");
    }

    #[test]
    fn kv_delete() {
        let db = Database::in_memory().unwrap();
        db.put_kv("key", b"value").unwrap();

        assert!(db.delete_kv("key").unwrap());
        assert!(db.get_kv("key").unwrap().is_none());
    }

    #[test]
    fn kv_delete_missing_returns_false() {
        let db = Database::in_memory().unwrap();
        assert!(!db.delete_kv("nonexistent").unwrap());
    }

    #[test]
    fn store_and_get_identity() {
        let db = Database::in_memory().unwrap();
        let doc = r#"{"id": "did:key:z123"}"#;
        let key = [42u8; 32];

        db.store_identity("did:key:z123", doc, Some(&key)).unwrap();
        let (stored_doc, stored_key) = db.get_identity("did:key:z123").unwrap().unwrap();

        assert_eq!(stored_doc, doc);
        assert_eq!(stored_key.unwrap(), key);
    }

    #[test]
    fn get_missing_identity_returns_none() {
        let db = Database::in_memory().unwrap();
        assert!(db.get_identity("did:key:znothing").unwrap().is_none());
    }

    #[test]
    fn file_database() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");

        {
            let db = Database::open(&path).unwrap();
            db.put_kv("persist", b"data").unwrap();
        }

        {
            let db = Database::open(&path).unwrap();
            let value = db.get_kv("persist").unwrap().unwrap();
            assert_eq!(value, b"data");
        }
    }
}
