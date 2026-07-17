//! SQLite (WAL) storage for E-OS Notes.
//!
//! One connection, synchronous access — the app is single-threaded (Slint event
//! loop) and SQLite in WAL mode makes every small autosave cheap and durable.

use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Db {
    conn: Connection,
}

#[derive(Clone, Debug)]
pub struct NoteMeta {
    pub id: i64,
    pub title: String,
    pub updated_at: i64,
}

#[derive(Clone, Debug)]
pub struct Note {
    pub id: i64,
    pub title: String,
    pub body: String,
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Default database location: `$HOME/.local/share/eos-notes/notes.db`,
/// falling back to `/tmp/eos-notes.db` when HOME is unset.
pub fn default_path() -> PathBuf {
    match std::env::var_os("HOME") {
        Some(home) => Path::new(&home)
            .join(".local")
            .join("share")
            .join("eos-notes")
            .join("notes.db"),
        None => PathBuf::from("/tmp/eos-notes.db"),
    }
}

impl Db {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        if let Some(dir) = path.parent() {
            // Best effort — open() below reports the real error if this failed.
            let _ = std::fs::create_dir_all(dir);
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS notes (
                id         INTEGER PRIMARY KEY,
                title      TEXT NOT NULL DEFAULT '',
                body       TEXT NOT NULL DEFAULT '',
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )",
            [],
        )?;
        Ok(Db { conn })
    }

    pub fn journal_mode(&self) -> rusqlite::Result<String> {
        self.conn.query_row("PRAGMA journal_mode", [], |r| r.get(0))
    }

    /// Notes ordered by last edit, newest first; `filter` does a substring
    /// match on title and body (empty filter lists everything).
    pub fn list(&self, filter: &str) -> rusqlite::Result<Vec<NoteMeta>> {
        let like = format!("%{}%", filter);
        let mut stmt = self.conn.prepare(
            "SELECT id, title, updated_at FROM notes
             WHERE title LIKE ?1 OR body LIKE ?1
             ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map(params![like], |r| {
            Ok(NoteMeta {
                id: r.get(0)?,
                title: r.get(1)?,
                updated_at: r.get(2)?,
            })
        })?;
        rows.collect()
    }

    pub fn get(&self, id: i64) -> rusqlite::Result<Note> {
        self.conn.query_row(
            "SELECT id, title, body FROM notes WHERE id = ?1",
            params![id],
            |r| {
                Ok(Note {
                    id: r.get(0)?,
                    title: r.get(1)?,
                    body: r.get(2)?,
                })
            },
        )
    }

    pub fn create(&self) -> rusqlite::Result<i64> {
        let t = now();
        self.conn.execute(
            "INSERT INTO notes (title, body, created_at, updated_at)
             VALUES ('', '', ?1, ?1)",
            params![t],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn save(&self, id: i64, title: &str, body: &str) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE notes SET title = ?2, body = ?3, updated_at = ?4 WHERE id = ?1",
            params![id, title, body, now()],
        )?;
        Ok(())
    }

    pub fn delete(&self, id: i64) -> rusqlite::Result<()> {
        self.conn
            .execute("DELETE FROM notes WHERE id = ?1", params![id])?;
        Ok(())
    }
}

/// Headless proof of the storage layer, run by `eos-notes --selftest`:
/// create → write → reopen → read back → delete, and assert WAL is active.
/// Prints `EOS-NOTES-SELFTEST-OK` on success (asserted from the boot serial).
pub fn selftest(path: &Path) -> Result<(), String> {
    let _ = std::fs::remove_file(path);

    let db = Db::open(path).map_err(|e| format!("open: {e}"))?;
    let mode = db
        .journal_mode()
        .map_err(|e| format!("journal_mode: {e}"))?;
    if mode.to_lowercase() != "wal" {
        return Err(format!("journal_mode is '{mode}', expected 'wal'"));
    }
    let id = db.create().map_err(|e| format!("create: {e}"))?;
    db.save(id, "selftest", "zawartość notatki — E-OS")
        .map_err(|e| format!("save: {e}"))?;
    drop(db);

    let db = Db::open(path).map_err(|e| format!("reopen: {e}"))?;
    let note = db.get(id).map_err(|e| format!("get: {e}"))?;
    if note.title != "selftest" || note.body != "zawartość notatki — E-OS" {
        return Err(format!("readback mismatch: {:?}", note));
    }
    let listed = db.list("zawartość").map_err(|e| format!("list: {e}"))?;
    if listed.len() != 1 || listed[0].id != id {
        return Err(format!("search mismatch: {:?}", listed));
    }
    db.delete(id).map_err(|e| format!("delete: {e}"))?;
    if db.get(id).is_ok() {
        return Err("note still present after delete".into());
    }
    Ok(())
}
