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
                CREATE INDEX IF NOT EXISTS idx_reputation_subject ON reputation_events(subject_did);

                CREATE TABLE IF NOT EXISTS ratchet_sessions (
                    session_id TEXT PRIMARY KEY,
                    our_did    TEXT NOT NULL,
                    peer_did   TEXT NOT NULL,
                    state      BLOB NOT NULL,
                    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
                );
                CREATE INDEX IF NOT EXISTS idx_ratchet_peer ON ratchet_sessions(peer_did);

                CREATE TABLE IF NOT EXISTS stored_messages (
                    id         TEXT PRIMARY KEY,
                    session_id TEXT NOT NULL,
                    sender_did TEXT NOT NULL,
                    payload    BLOB NOT NULL,
                    timestamp  TEXT NOT NULL,
                    FOREIGN KEY (session_id) REFERENCES ratchet_sessions(session_id)
                );
                CREATE INDEX IF NOT EXISTS idx_stored_messages_session ON stored_messages(session_id, timestamp);",
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

    // ---- Ratchet session persistence ----

    pub fn store_ratchet_session(
        &self,
        session_id: &str,
        our_did: &str,
        peer_did: &str,
        state: &[u8],
    ) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO ratchet_sessions (session_id, our_did, peer_did, state, updated_at)
                 VALUES (?1, ?2, ?3, ?4, datetime('now'))
                 ON CONFLICT(session_id) DO UPDATE SET state = ?4, updated_at = datetime('now')",
                params![session_id, our_did, peer_did, state],
            )
            .map_err(|e| Error::Storage(format!("store ratchet session failed: {e}")))?;
        Ok(())
    }

    pub fn get_ratchet_session(&self, session_id: &str) -> Result<Option<Vec<u8>>> {
        let mut stmt = self
            .conn
            .prepare("SELECT state FROM ratchet_sessions WHERE session_id = ?1")
            .map_err(|e| Error::Storage(format!("prepare failed: {e}")))?;

        let result = stmt
            .query_row(params![session_id], |row| row.get(0))
            .optional()
            .map_err(|e| Error::Storage(format!("get ratchet session failed: {e}")))?;

        Ok(result)
    }

    pub fn get_ratchet_session_by_peer(&self, peer_did: &str) -> Result<Option<(String, Vec<u8>)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT session_id, state FROM ratchet_sessions WHERE peer_did = ?1 ORDER BY updated_at DESC LIMIT 1")
            .map_err(|e| Error::Storage(format!("prepare failed: {e}")))?;

        let result = stmt
            .query_row(params![peer_did], |row| Ok((row.get(0)?, row.get(1)?)))
            .optional()
            .map_err(|e| Error::Storage(format!("get ratchet by peer failed: {e}")))?;

        Ok(result)
    }

    pub fn delete_ratchet_session(&self, session_id: &str) -> Result<bool> {
        let count = self
            .conn
            .execute(
                "DELETE FROM ratchet_sessions WHERE session_id = ?1",
                params![session_id],
            )
            .map_err(|e| Error::Storage(format!("delete ratchet session failed: {e}")))?;
        Ok(count > 0)
    }

    pub fn list_ratchet_sessions(&self, our_did: &str) -> Result<Vec<(String, String)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT session_id, peer_did FROM ratchet_sessions WHERE our_did = ?1 ORDER BY updated_at DESC")
            .map_err(|e| Error::Storage(format!("prepare failed: {e}")))?;

        let rows = stmt
            .query_map(params![our_did], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| Error::Storage(format!("list ratchet sessions failed: {e}")))?;

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| Error::Storage(format!("collect sessions failed: {e}")))
    }

    // ---- Stored messages ----

    pub fn store_message(
        &self,
        id: &str,
        session_id: &str,
        sender_did: &str,
        payload: &[u8],
        timestamp: &str,
    ) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO stored_messages (id, session_id, sender_did, payload, timestamp)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id, session_id, sender_did, payload, timestamp],
            )
            .map_err(|e| Error::Storage(format!("store message failed: {e}")))?;
        Ok(())
    }

    #[allow(clippy::type_complexity)]
    pub fn get_messages(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<(String, String, Vec<u8>, String)>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, sender_did, payload, timestamp FROM stored_messages
                 WHERE session_id = ?1 ORDER BY timestamp DESC LIMIT ?2",
            )
            .map_err(|e| Error::Storage(format!("prepare failed: {e}")))?;

        let rows = stmt
            .query_map(params![session_id, limit as i64], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })
            .map_err(|e| Error::Storage(format!("get messages failed: {e}")))?;

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| Error::Storage(format!("collect messages failed: {e}")))
    }

    // ---- Identity ----

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
    fn store_and_get_ratchet_session() {
        let db = Database::in_memory().unwrap();
        let state = b"ratchet-state-bytes";

        db.store_ratchet_session("sess1", "did:key:zme", "did:key:zpeer", state)
            .unwrap();

        let loaded = db.get_ratchet_session("sess1").unwrap().unwrap();
        assert_eq!(loaded, state);
    }

    #[test]
    fn get_ratchet_session_by_peer() {
        let db = Database::in_memory().unwrap();
        db.store_ratchet_session("sess1", "did:key:zme", "did:key:zpeer", b"state1")
            .unwrap();

        let (id, state) = db
            .get_ratchet_session_by_peer("did:key:zpeer")
            .unwrap()
            .unwrap();
        assert_eq!(id, "sess1");
        assert_eq!(state, b"state1");
    }

    #[test]
    fn ratchet_session_upsert() {
        let db = Database::in_memory().unwrap();
        db.store_ratchet_session("sess1", "did:key:zme", "did:key:zpeer", b"v1")
            .unwrap();
        db.store_ratchet_session("sess1", "did:key:zme", "did:key:zpeer", b"v2")
            .unwrap();

        let loaded = db.get_ratchet_session("sess1").unwrap().unwrap();
        assert_eq!(loaded, b"v2");
    }

    #[test]
    fn delete_ratchet_session() {
        let db = Database::in_memory().unwrap();
        db.store_ratchet_session("sess1", "did:key:zme", "did:key:zpeer", b"state")
            .unwrap();
        assert!(db.delete_ratchet_session("sess1").unwrap());
        assert!(db.get_ratchet_session("sess1").unwrap().is_none());
    }

    #[test]
    fn list_ratchet_sessions() {
        let db = Database::in_memory().unwrap();
        db.store_ratchet_session("s1", "did:key:zme", "did:key:za", b"a")
            .unwrap();
        db.store_ratchet_session("s2", "did:key:zme", "did:key:zb", b"b")
            .unwrap();
        db.store_ratchet_session("s3", "did:key:zother", "did:key:zc", b"c")
            .unwrap();

        let sessions = db.list_ratchet_sessions("did:key:zme").unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn store_and_get_messages() {
        let db = Database::in_memory().unwrap();
        db.store_ratchet_session("sess1", "did:key:zme", "did:key:zpeer", b"state")
            .unwrap();

        db.store_message(
            "msg1",
            "sess1",
            "did:key:zme",
            b"hello",
            "2026-01-01T00:00:00Z",
        )
        .unwrap();
        db.store_message(
            "msg2",
            "sess1",
            "did:key:zpeer",
            b"hi",
            "2026-01-01T00:01:00Z",
        )
        .unwrap();

        let messages = db.get_messages("sess1", 10).unwrap();
        assert_eq!(messages.len(), 2);
    }

    #[test]
    fn get_messages_respects_limit() {
        let db = Database::in_memory().unwrap();
        db.store_ratchet_session("sess1", "did:key:zme", "did:key:zpeer", b"state")
            .unwrap();

        for i in 0..5 {
            db.store_message(
                &format!("msg{i}"),
                "sess1",
                "did:key:zme",
                format!("msg {i}").as_bytes(),
                &format!("2026-01-01T00:0{i}:00Z"),
            )
            .unwrap();
        }

        let messages = db.get_messages("sess1", 2).unwrap();
        assert_eq!(messages.len(), 2);
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
