//! Calculator history in SQLite, for every platform the core runs on.
//!
//! This is the only crate that knows SQLite exists. `calc_core` sees a
//! [`HistoryStore`]; the front ends decide where the file goes (an app
//! sandbox on Apple and Android, an OPFS-backed VFS in the browser) and hand
//! the path to [`SqliteHistory::open`].
//!
//! On `wasm32-unknown-unknown` there is no libsqlite3 to link, so `rusqlite`
//! builds against `sqlite-wasm-rs`, which compiles the same amalgamation to
//! wasm. Nothing in this file is conditional on the target: the VFS is
//! whatever the caller installed before opening.

use calc_core::{Calculator, Entry, HISTORY_LIMIT, HistoryStore, MemoryHistory, StoreError};
use rusqlite::{Connection, OptionalExtension};
use std::fmt::Display;

/// The schema version this crate writes, kept in SQLite's own
/// `user_version`, so an older file can be migrated instead of rejected.
const SCHEMA_VERSION: i64 = 1;

/// [`HISTORY_LIMIT`] as SQLite sees it: `usize` is not one of its types.
const ROW_LIMIT: i64 = HISTORY_LIMIT as i64;

/// Evaluated lines kept in a SQLite database.
#[derive(Debug)]
pub struct SqliteHistory {
    conn: Connection,
    description: String,
}

impl SqliteHistory {
    /// Opens the database at `path`, creating the file and schema if needed.
    pub fn open(path: &str) -> Result<Self, StoreError> {
        Self::from_connection(Connection::open(path).map_err(to_store_error)?, "file")
    }

    /// A database held in SQLite's own memory. The fallback when a front end
    /// has nowhere to put a file, and what the tests use.
    pub fn in_memory() -> Result<Self, StoreError> {
        Self::from_connection(Connection::open_in_memory().map_err(to_store_error)?, "memory")
    }

    /// Wraps a connection opened elsewhere -- by a front end that had to
    /// install a VFS first, say -- and applies the schema.
    ///
    /// `storage` says where the bytes are going ("file", "OPFS"); it ends up
    /// in the footer caption of every front end, so write it for a reader.
    pub fn from_connection(conn: Connection, storage: &str) -> Result<Self, StoreError> {
        let store =
            SqliteHistory { conn, description: format!("SQLite {} ({storage})", rusqlite::version()) };
        store.migrate()?;
        Ok(store)
    }

    /// Brings an empty or older database up to [`SCHEMA_VERSION`].
    ///
    /// `AUTOINCREMENT` is load-bearing: ids must not be reused after pruning,
    /// because a renderer keys its history rows on them.
    fn migrate(&self) -> Result<(), StoreError> {
        let version: i64 =
            self.conn.query_row("PRAGMA user_version", [], |r| r.get(0)).map_err(to_store_error)?;
        if version >= SCHEMA_VERSION {
            return Ok(());
        }
        self.conn
            .execute_batch(&format!(
                "BEGIN;
                 CREATE TABLE IF NOT EXISTS history (
                     id         INTEGER PRIMARY KEY AUTOINCREMENT,
                     expression TEXT NOT NULL,
                     result     TEXT NOT NULL,
                     created_at INTEGER NOT NULL DEFAULT (unixepoch())
                 );
                 PRAGMA user_version = {SCHEMA_VERSION};
                 COMMIT;"
            ))
            .map_err(to_store_error)
    }
}

impl HistoryStore for SqliteHistory {
    fn append(&mut self, expression: &str, result: &str) -> Result<u64, StoreError> {
        let tx = self.conn.transaction().map_err(to_store_error)?;
        tx.execute("INSERT INTO history (expression, result) VALUES (?1, ?2)", (expression, result))
            .map_err(to_store_error)?;
        let id = tx.last_insert_rowid();
        // Keep the newest HISTORY_LIMIT rows. Comparing against the id of the
        // oldest row worth keeping is one index scan, rather than a NOT IN
        // over a subquery.
        let oldest_kept: Option<i64> = tx
            .query_row("SELECT id FROM history ORDER BY id DESC LIMIT 1 OFFSET ?1", [ROW_LIMIT - 1], |r| {
                r.get(0)
            })
            .optional()
            .map_err(to_store_error)?;
        if let Some(oldest_kept) = oldest_kept {
            tx.execute("DELETE FROM history WHERE id < ?1", [oldest_kept]).map_err(to_store_error)?;
        }
        tx.commit().map_err(to_store_error)?;
        Ok(id as u64)
    }

    fn entries(&self) -> Result<Vec<Entry>, StoreError> {
        // The newest rows, then back into oldest-first order. Taking
        // `ORDER BY id ASC LIMIT n` instead would hand back the *oldest* n
        // from a database that somehow holds more than the cap -- a file
        // written when HISTORY_LIMIT was larger, say.
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, expression, result FROM
                   (SELECT id, expression, result FROM history ORDER BY id DESC LIMIT ?1)
                 ORDER BY id ASC",
            )
            .map_err(to_store_error)?;
        let rows = stmt
            .query_map([ROW_LIMIT], |row| {
                Ok(Entry { id: row.get::<_, i64>(0)? as u64, expression: row.get(1)?, result: row.get(2)? })
            })
            .map_err(to_store_error)?;
        rows.collect::<rusqlite::Result<Vec<Entry>>>().map_err(to_store_error)
    }

    fn clear(&mut self) -> Result<(), StoreError> {
        self.conn.execute("DELETE FROM history", []).map_err(to_store_error)?;
        Ok(())
    }

    fn description(&self) -> String {
        self.description.clone()
    }
}

/// Builds a calculator whose history is the SQLite database at `path`.
///
/// Every front end wants this and none of them should implement it: a
/// calculator that cannot open its database still has to add up, so an
/// unusable path degrades to an in-memory store carrying the reason, which
/// the footer caption then shows. Nothing here can fail.
pub fn open_calculator(path: &str) -> Calculator {
    let opened = SqliteHistory::open(path)
        .map(|store| Box::new(store) as Box<dyn HistoryStore>)
        .and_then(Calculator::with_store);
    match opened {
        Ok(calculator) => calculator,
        Err(e) => Calculator::with_store(Box::new(MemoryHistory::fallback(e.message)))
            .expect("the memory store always reads back"),
    }
}

/// SQLite's errors are not actionable here, so they travel as text. The
/// message is what a front end shows when history cannot be saved.
fn to_store_error(e: impl Display) -> StoreError {
    StoreError::new(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use calc_core::Calculator;

    /// A path in the test's own temp dir, so cases cannot collide.
    fn db_path(name: &str) -> String {
        let dir = std::env::var("TEST_TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
        format!("{dir}/{name}.db")
    }

    #[test]
    fn rows_round_trip() {
        let mut store = SqliteHistory::in_memory().unwrap();
        assert_eq!(store.append("6*7", "42").unwrap(), 1);
        assert_eq!(store.append("ans/2", "21").unwrap(), 2);
        assert_eq!(
            store.entries().unwrap(),
            vec![
                Entry { id: 1, expression: "6*7".into(), result: "42".into() },
                Entry { id: 2, expression: "ans/2".into(), result: "21".into() },
            ]
        );
    }

    #[test]
    fn the_description_names_the_linked_sqlite() {
        let store = SqliteHistory::in_memory().unwrap();
        let description = store.description();
        assert!(description.starts_with("SQLite 3."), "{description}");
        assert!(description.ends_with(" (memory)"), "{description}");
    }

    #[test]
    fn history_survives_reopening_the_file() {
        let path = db_path("reopen");
        let mut store = SqliteHistory::open(&path).unwrap();
        store.append("6*7", "42").unwrap();
        drop(store);

        let store = SqliteHistory::open(&path).unwrap();
        assert_eq!(
            store.entries().unwrap(),
            vec![Entry { id: 1, expression: "6*7".into(), result: "42".into() }]
        );
    }

    #[test]
    fn a_reopened_calculator_keeps_its_history_and_ans() {
        let path = db_path("ans");
        let mut calc = Calculator::with_store(Box::new(SqliteHistory::open(&path).unwrap())).unwrap();
        assert_eq!(calc.evaluate("6*7").unwrap(), "42");

        let mut calc = Calculator::with_store(Box::new(SqliteHistory::open(&path).unwrap())).unwrap();
        assert_eq!(calc.history().len(), 1);
        assert_eq!(calc.last_result(), Some("42"));
        assert_eq!(calc.evaluate("ans+1").unwrap(), "43", "`ans` carries across a restart");
        assert!(calc.store_error().is_none());
    }

    #[test]
    fn open_calculator_uses_the_file_and_keeps_its_history() {
        let path = db_path("open_calculator");
        let mut calc = open_calculator(&path);
        assert!(calc.store_description().starts_with("SQLite 3."), "{}", calc.store_description());
        calc.evaluate("6*7").unwrap();

        let calc = open_calculator(&path);
        assert_eq!(calc.history().len(), 1);
        assert!(calc.store_error().is_none());
    }

    #[test]
    fn an_unusable_path_falls_back_to_memory_with_the_reason() {
        // A directory that does not exist: SQLite cannot create the file.
        let calc = open_calculator(&db_path("no/such/dir/history"));
        assert!(calc.history().is_empty());
        let description = calc.store_description();
        assert!(description.starts_with("memory ("), "{description}");
        assert!(description.contains("unable to open database file"), "{description}");
    }

    #[test]
    fn a_fractional_ans_survives_a_restart() {
        let path = db_path("fractional");
        let mut calc = Calculator::with_store(Box::new(SqliteHistory::open(&path).unwrap())).unwrap();
        assert_eq!(calc.evaluate("1/8").unwrap(), "0.125");

        let mut calc = Calculator::with_store(Box::new(SqliteHistory::open(&path).unwrap())).unwrap();
        assert_eq!(calc.evaluate("ans*8").unwrap(), "1");
    }

    #[test]
    fn history_is_capped_and_ids_are_never_reused() {
        let mut store = SqliteHistory::in_memory().unwrap();
        for i in 0..HISTORY_LIMIT + 10 {
            store.append(&format!("{i}+0"), &i.to_string()).unwrap();
        }
        let entries = store.entries().unwrap();
        assert_eq!(entries.len(), HISTORY_LIMIT);
        assert_eq!(entries.first().unwrap().expression, "10+0", "the oldest rows are deleted");
        assert_eq!(entries.last().unwrap().id, HISTORY_LIMIT as u64 + 10);

        store.clear().unwrap();
        assert!(store.entries().unwrap().is_empty());
        assert_eq!(
            store.append("1", "1").unwrap(),
            HISTORY_LIMIT as u64 + 11,
            "AUTOINCREMENT keeps ids unique across pruning and clearing"
        );
    }

    #[test]
    fn an_overfull_database_reads_back_its_newest_rows() {
        // Rows inserted behind `append`'s back, as a file written when the
        // cap was higher would hold.
        let store = SqliteHistory::in_memory().unwrap();
        for i in 0..HISTORY_LIMIT + 5 {
            store
                .conn
                .execute("INSERT INTO history (expression, result) VALUES (?1, ?2)", (i.to_string(), "0"))
                .unwrap();
        }
        let entries = store.entries().unwrap();
        assert_eq!(entries.len(), HISTORY_LIMIT);
        assert_eq!(entries.first().unwrap().expression, "5", "the newest rows, not the oldest");
        assert_eq!(entries.last().unwrap().expression, (HISTORY_LIMIT + 4).to_string());
    }

    #[test]
    fn an_existing_database_is_migrated_only_once() {
        let path = db_path("migrate");
        SqliteHistory::open(&path).unwrap().append("1+1", "2").unwrap();
        let store = SqliteHistory::open(&path).unwrap();
        assert_eq!(store.entries().unwrap().len(), 1, "reopening must not recreate the table");
        let version: i64 = store.conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(version, SCHEMA_VERSION);
    }
}
