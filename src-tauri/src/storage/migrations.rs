//! Schema migrations, applied in order according to `PRAGMA user_version`.
//!
//! Rules: never edit a migration that has shipped; append a new one instead.
//! Each migration runs in its own transaction together with the version bump,
//! so a failure leaves the database at the previous version.

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

pub const MIGRATIONS: &[&str] = &[
    // v1 — initial schema
    r#"
    CREATE TABLE settings (
        id      INTEGER PRIMARY KEY CHECK (id = 1),
        version INTEGER NOT NULL,
        data    TEXT    NOT NULL
    );
    CREATE TABLE app_state (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );
    CREATE TABLE profiles (
        id          TEXT PRIMARY KEY,
        name        TEXT    NOT NULL,
        description TEXT    NOT NULL DEFAULT '',
        spec        TEXT    NOT NULL,
        sort_index  INTEGER NOT NULL DEFAULT 0,
        created_at  INTEGER NOT NULL,
        updated_at  INTEGER NOT NULL
    );
    CREATE TABLE presets (
        id           TEXT PRIMARY KEY,
        name         TEXT    NOT NULL,
        category     TEXT    NOT NULL,
        spec         TEXT    NOT NULL,
        favorite     INTEGER NOT NULL DEFAULT 0,
        builtin      INTEGER NOT NULL DEFAULT 0,
        created_at   INTEGER NOT NULL,
        last_used_at INTEGER
    );
    CREATE TABLE shortcuts (
        action      TEXT PRIMARY KEY,
        accelerator TEXT    NOT NULL,
        enabled     INTEGER NOT NULL DEFAULT 1
    );
    CREATE TABLE recents (
        id        INTEGER PRIMARY KEY AUTOINCREMENT,
        name      TEXT    NOT NULL,
        source    TEXT    NOT NULL,
        source_id TEXT,
        spec      TEXT    NOT NULL,
        used_at   INTEGER NOT NULL
    );
    CREATE INDEX idx_recents_used_at ON recents (used_at DESC);
    CREATE INDEX idx_profiles_sort ON profiles (sort_index, created_at);
    "#,
];

pub fn latest_version() -> i64 {
    MIGRATIONS.len() as i64
}

pub fn current_version(conn: &Connection) -> AppResult<i64> {
    Ok(conn.query_row("PRAGMA user_version", [], |r| r.get(0))?)
}

pub fn run(conn: &mut Connection) -> AppResult<()> {
    let current = current_version(conn)?;
    if current > latest_version() {
        return Err(AppError::System(format!(
            "The database was created by a newer version of Crosshair Studio (schema v{current}, this version supports v{}).",
            latest_version()
        )));
    }
    for (index, sql) in MIGRATIONS.iter().enumerate().skip(current as usize) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", (index + 1) as i64)?;
        tx.commit()?;
    }
    Ok(())
}
