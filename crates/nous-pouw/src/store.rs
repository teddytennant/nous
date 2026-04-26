//! SQLite-backed persistence for [`ChainState`] and finalized [`Block`]s.
//!
//! The consensus engine remains a pure in-memory state machine; this module
//! provides an out-of-band durability layer so a node can crash and reload
//! its chain state from disk on next boot.
//!
//! Each `save_block` performs a single SQL transaction that:
//!   1. inserts the block (idempotent via `INSERT OR IGNORE` on the unique
//!      `(height, hash)` pair),
//!   2. replaces the singleton `state_snapshot` row,
//!   3. wipes & rewrites the `workers`, `used_jobs`, and `used_worker_jobs`
//!      tables to match the supplied [`ChainState`].
//!
//! On open, WAL journal mode is enabled for crash safety + concurrent reads.
//!
//! The `finalized` map inside [`ChainState`] is re-derived from the `blocks`
//! table on load (it's just `height -> hash`), so we don't need a separate
//! table for it.

use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};

use crate::block::{Block, BlockHash, BlockHeight};
use crate::envelope::JobId;
use crate::state::{ChainState, WorkerId, WorkerInfo};

/// A SQLite-backed persistent store for the PoUW chain.
pub struct Store {
    conn: Connection,
}

#[derive(thiserror::Error, Debug)]
pub enum StoreError {
    #[error("sqlite: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("encoding: {0}")]
    Encoding(#[from] serde_json::Error),
}

const SCHEMA: &str = r"
CREATE TABLE IF NOT EXISTS blocks (
  height INTEGER PRIMARY KEY,
  hash   BLOB    NOT NULL UNIQUE,
  body   BLOB    NOT NULL
);

CREATE TABLE IF NOT EXISTS state_snapshot (
  id           INTEGER PRIMARY KEY CHECK (id = 1),
  height       INTEGER NOT NULL,
  head_hash    BLOB    NOT NULL,
  total_supply INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS workers (
  worker_id BLOB    PRIMARY KEY,
  stake     INTEGER NOT NULL,
  balance   INTEGER NOT NULL,
  trust     REAL    NOT NULL,
  slashed   INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS used_jobs (
  job_id BLOB PRIMARY KEY
);

CREATE TABLE IF NOT EXISTS used_worker_jobs (
  worker_id BLOB NOT NULL,
  job_id    BLOB NOT NULL,
  PRIMARY KEY (worker_id, job_id)
);

CREATE INDEX IF NOT EXISTS idx_blocks_hash ON blocks(hash);
";

impl Store {
    /// Open or create the SQLite database at `path`. Runs migrations on first
    /// open and enables WAL journal mode.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        let conn = Connection::open(path)?;
        Self::init(conn)
    }

    /// In-memory variant (useful for tests). WAL is not enabled because it's
    /// meaningless for `:memory:`.
    pub fn open_in_memory() -> Result<Self, StoreError> {
        let conn = Connection::open_in_memory()?;
        // Skip WAL pragma — meaningless for memory-backed DBs.
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn })
    }

    fn init(conn: Connection) -> Result<Self, StoreError> {
        // WAL gives us crash-safe writes + concurrent readers. Best-effort —
        // some filesystems (e.g. NFS) reject WAL but the store still works.
        let _: Result<String, _> =
            conn.query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0));
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn })
    }

    /// Persist a finalized block plus the resulting state snapshot in one
    /// atomic transaction. Re-saving the same block (same height & hash) is
    /// a no-op and does not error.
    pub fn save_block(&mut self, block: &Block, state: &ChainState) -> Result<(), StoreError> {
        let body = serde_json::to_vec(block)?;
        let block_hash = block.hash();
        let block_height = block.header.height as i64;

        let tx = self.conn.transaction()?;

        // Idempotent insert. If a row with the same height already exists we
        // leave it alone — re-saves are cheap and never error.
        tx.execute(
            "INSERT OR IGNORE INTO blocks (height, hash, body) VALUES (?1, ?2, ?3)",
            params![block_height, &block_hash[..], body],
        )?;

        // State snapshot is a single row keyed at id=1. Replace wholesale.
        tx.execute(
            "INSERT OR REPLACE INTO state_snapshot (id, height, head_hash, total_supply) \
             VALUES (1, ?1, ?2, ?3)",
            params![
                state.height as i64,
                &state.head_hash[..],
                state.total_supply as i64,
            ],
        )?;

        // Workers, used_jobs, and used_worker_jobs are wholesale-replaced
        // each save: the in-memory ChainState is the source of truth, and
        // these tables are denormalized projections of it.
        tx.execute("DELETE FROM workers", [])?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO workers (worker_id, stake, balance, trust, slashed) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )?;
            for (worker_id, info) in &state.workers {
                stmt.execute(params![
                    &worker_id.0[..],
                    info.stake as i64,
                    info.balance as i64,
                    info.trust,
                    info.slashed as i64,
                ])?;
            }
        }

        tx.execute("DELETE FROM used_jobs", [])?;
        {
            let mut stmt = tx.prepare("INSERT INTO used_jobs (job_id) VALUES (?1)")?;
            for job in &state.used_jobs {
                stmt.execute(params![&job.0[..]])?;
            }
        }

        tx.execute("DELETE FROM used_worker_jobs", [])?;
        {
            let mut stmt =
                tx.prepare("INSERT INTO used_worker_jobs (worker_id, job_id) VALUES (?1, ?2)")?;
            for (worker, job) in &state.used_worker_jobs {
                stmt.execute(params![&worker.0[..], &job.0[..]])?;
            }
        }

        tx.commit()?;
        Ok(())
    }

    /// Load the latest persisted [`ChainState`]. Returns the default empty
    /// state if no snapshot has ever been written.
    pub fn load_state(&self) -> Result<ChainState, StoreError> {
        let snapshot: Option<(i64, Vec<u8>, i64)> = self
            .conn
            .query_row(
                "SELECT height, head_hash, total_supply FROM state_snapshot WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;

        let Some((height, head_hash_vec, total_supply)) = snapshot else {
            return Ok(ChainState::default());
        };

        let mut state = ChainState {
            height: height as u64,
            head_hash: blob_to_hash(&head_hash_vec),
            total_supply: total_supply as u64,
            ..ChainState::default()
        };

        // Workers
        let mut stmt = self
            .conn
            .prepare("SELECT worker_id, stake, balance, trust, slashed FROM workers")?;
        let rows = stmt.query_map([], |row| {
            let id_blob: Vec<u8> = row.get(0)?;
            let stake: i64 = row.get(1)?;
            let balance: i64 = row.get(2)?;
            let trust: f64 = row.get(3)?;
            let slashed: i64 = row.get(4)?;
            Ok((
                WorkerId(blob_to_hash(&id_blob)),
                WorkerInfo {
                    stake: stake as u64,
                    balance: balance as u64,
                    trust,
                    slashed: slashed != 0,
                    nonce: 0,
                },
            ))
        })?;
        for row in rows {
            let (id, info) = row?;
            state.workers.insert(id, info);
        }

        // Used jobs
        let mut stmt = self.conn.prepare("SELECT job_id FROM used_jobs")?;
        let rows = stmt.query_map([], |row| {
            let blob: Vec<u8> = row.get(0)?;
            Ok(JobId(blob_to_hash(&blob)))
        })?;
        for row in rows {
            state.used_jobs.insert(row?);
        }

        // Used (worker, job) pairs
        let mut stmt = self
            .conn
            .prepare("SELECT worker_id, job_id FROM used_worker_jobs")?;
        let rows = stmt.query_map([], |row| {
            let w: Vec<u8> = row.get(0)?;
            let j: Vec<u8> = row.get(1)?;
            Ok((WorkerId(blob_to_hash(&w)), JobId(blob_to_hash(&j))))
        })?;
        for row in rows {
            state.used_worker_jobs.insert(row?);
        }

        // Re-derive `finalized` from the blocks table.
        let mut stmt = self.conn.prepare("SELECT height, hash FROM blocks")?;
        let rows = stmt.query_map([], |row| {
            let h: i64 = row.get(0)?;
            let hash: Vec<u8> = row.get(1)?;
            Ok((h as u64, blob_to_hash(&hash)))
        })?;
        for row in rows {
            let (h, hash) = row?;
            state.finalized.insert(h, hash);
        }

        Ok(state)
    }

    /// Iterate finalized blocks in ascending height order.
    pub fn iter_blocks(&self) -> Result<Vec<Block>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT body FROM blocks ORDER BY height ASC")?;
        let rows = stmt.query_map([], |row| {
            let body: Vec<u8> = row.get(0)?;
            Ok(body)
        })?;
        let mut out = Vec::new();
        for row in rows {
            let body = row?;
            out.push(serde_json::from_slice(&body)?);
        }
        Ok(out)
    }

    /// Look up a block by height.
    pub fn block_at(&self, height: BlockHeight) -> Result<Option<Block>, StoreError> {
        let body: Option<Vec<u8>> = self
            .conn
            .query_row(
                "SELECT body FROM blocks WHERE height = ?1",
                params![height as i64],
                |row| row.get(0),
            )
            .optional()?;
        match body {
            Some(b) => Ok(Some(serde_json::from_slice(&b)?)),
            None => Ok(None),
        }
    }

    /// Look up a block by its hash.
    pub fn block_by_hash(&self, hash: &BlockHash) -> Result<Option<Block>, StoreError> {
        let body: Option<Vec<u8>> = self
            .conn
            .query_row(
                "SELECT body FROM blocks WHERE hash = ?1",
                params![&hash[..]],
                |row| row.get(0),
            )
            .optional()?;
        match body {
            Some(b) => Ok(Some(serde_json::from_slice(&b)?)),
            None => Ok(None),
        }
    }

    /// Latest finalized height (0 if no blocks have been saved).
    pub fn head_height(&self) -> Result<BlockHeight, StoreError> {
        let h: Option<i64> = self
            .conn
            .query_row(
                "SELECT MAX(height) FROM blocks",
                [],
                |row| row.get::<_, Option<i64>>(0),
            )
            .optional()?
            .flatten();
        Ok(h.map(|x| x as BlockHeight).unwrap_or(0))
    }
}

/// Convert a database BLOB into a fixed 32-byte hash. Defensive — pads with
/// zeros or truncates if the BLOB is the wrong width (which should never
/// happen for data we wrote ourselves).
fn blob_to_hash(blob: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    let n = blob.len().min(32);
    out[..n].copy_from_slice(&blob[..n]);
    out
}
