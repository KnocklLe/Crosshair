//! SQLite persistence (rusqlite, bundled SQLite).
//!
//! The storage layer only maps rows to domain structs. Business rules live in
//! the services (`core.rs`). Specs are stored as JSON and decoded leniently:
//! a corrupted row falls back to a default spec instead of breaking the app.

pub mod migrations;

use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::crosshair::presets::PresetCategory;
use crate::crosshair::validate::validate_spec;
use crate::crosshair::CrosshairSpec;
use crate::error::AppResult;
use crate::library::{Preset, RecentEntry, RECENTS_KEPT};
use crate::profiles::Profile;

pub struct Storage {
    conn: Connection,
    path: Option<PathBuf>,
}

pub const DB_FILE_NAME: &str = "crosshair-studio.db";

/// Decodes a stored spec; invalid or corrupted data gives the default spec.
pub fn decode_spec(json: &str) -> CrosshairSpec {
    match serde_json::from_str::<CrosshairSpec>(json) {
        Ok(spec) if validate_spec(&spec).is_ok() => spec,
        _ => CrosshairSpec::default(),
    }
}

fn encode_spec(spec: &CrosshairSpec) -> AppResult<String> {
    Ok(serde_json::to_string(spec)?)
}

fn row_to_profile(row: &Row) -> rusqlite::Result<Profile> {
    let spec: String = row.get("spec")?;
    Ok(Profile {
        id: row.get("id")?,
        name: row.get("name")?,
        description: row.get("description")?,
        spec: decode_spec(&spec),
        sort_index: row.get("sort_index")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn row_to_preset(row: &Row) -> rusqlite::Result<Preset> {
    let spec: String = row.get("spec")?;
    let category: String = row.get("category")?;
    Ok(Preset {
        id: row.get("id")?,
        name: row.get("name")?,
        category: PresetCategory::parse(&category).unwrap_or(PresetCategory::Custom),
        spec: decode_spec(&spec),
        favorite: row.get::<_, i64>("favorite")? != 0,
        builtin: row.get::<_, i64>("builtin")? != 0,
        created_at: row.get("created_at")?,
        last_used_at: row.get("last_used_at")?,
    })
}

impl Storage {
    pub fn open(path: &Path) -> AppResult<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let conn = Connection::open(path)?;
        Self::init(conn, Some(path.to_path_buf()))
    }

    pub fn open_in_memory() -> AppResult<Self> {
        Self::init(Connection::open_in_memory()?, None)
    }

    fn init(mut conn: Connection, path: Option<PathBuf>) -> AppResult<Self> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.busy_timeout(std::time::Duration::from_secs(3))?;
        migrations::run(&mut conn)?;
        Ok(Self { conn, path })
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn schema_version(&self) -> AppResult<i64> {
        migrations::current_version(&self.conn)
    }

    // ── key/value app state ───────────────────────────────────────────────

    pub fn get_value(&self, key: &str) -> AppResult<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT value FROM app_state WHERE key = ?1", [key], |r| r.get(0))
            .optional()?)
    }

    pub fn set_value(&self, key: &str, value: &str) -> AppResult<()> {
        self.conn.execute(
            "INSERT INTO app_state (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    // ── settings ──────────────────────────────────────────────────────────

    pub fn load_settings_json(&self) -> AppResult<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT data FROM settings WHERE id = 1", [], |r| r.get(0))
            .optional()?)
    }

    pub fn save_settings_json(&self, version: u32, json: &str) -> AppResult<()> {
        self.conn.execute(
            "INSERT INTO settings (id, version, data) VALUES (1, ?1, ?2)
             ON CONFLICT(id) DO UPDATE SET version = excluded.version, data = excluded.data",
            params![version, json],
        )?;
        Ok(())
    }

    // ── profiles ──────────────────────────────────────────────────────────

    pub fn list_profiles(&self) -> AppResult<Vec<Profile>> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM profiles ORDER BY sort_index ASC, created_at ASC")?;
        let rows = stmt.query_map([], row_to_profile)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn get_profile(&self, id: &str) -> AppResult<Option<Profile>> {
        Ok(self
            .conn
            .query_row("SELECT * FROM profiles WHERE id = ?1", [id], row_to_profile)
            .optional()?)
    }

    pub fn count_profiles(&self) -> AppResult<i64> {
        Ok(self.conn.query_row("SELECT COUNT(*) FROM profiles", [], |r| r.get(0))?)
    }

    pub fn next_profile_sort_index(&self) -> AppResult<i64> {
        Ok(self
            .conn
            .query_row("SELECT COALESCE(MAX(sort_index), -1) + 1 FROM profiles", [], |r| r.get(0))?)
    }

    pub fn insert_profile(&self, p: &Profile) -> AppResult<()> {
        self.conn.execute(
            "INSERT INTO profiles (id, name, description, spec, sort_index, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![p.id, p.name, p.description, encode_spec(&p.spec)?, p.sort_index, p.created_at, p.updated_at],
        )?;
        Ok(())
    }

    pub fn update_profile(&self, p: &Profile) -> AppResult<bool> {
        let n = self.conn.execute(
            "UPDATE profiles SET name = ?2, description = ?3, spec = ?4, sort_index = ?5, updated_at = ?6
             WHERE id = ?1",
            params![p.id, p.name, p.description, encode_spec(&p.spec)?, p.sort_index, p.updated_at],
        )?;
        Ok(n > 0)
    }

    pub fn update_profile_spec(&self, id: &str, spec: &CrosshairSpec, updated_at: i64) -> AppResult<bool> {
        let n = self.conn.execute(
            "UPDATE profiles SET spec = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, encode_spec(spec)?, updated_at],
        )?;
        Ok(n > 0)
    }

    pub fn delete_profile(&self, id: &str) -> AppResult<bool> {
        Ok(self.conn.execute("DELETE FROM profiles WHERE id = ?1", [id])? > 0)
    }

    // ── presets ───────────────────────────────────────────────────────────

    pub fn list_presets(&self) -> AppResult<Vec<Preset>> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM presets ORDER BY builtin DESC, created_at ASC, name ASC")?;
        let rows = stmt.query_map([], row_to_preset)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn get_preset(&self, id: &str) -> AppResult<Option<Preset>> {
        Ok(self
            .conn
            .query_row("SELECT * FROM presets WHERE id = ?1", [id], row_to_preset)
            .optional()?)
    }

    pub fn count_presets(&self) -> AppResult<i64> {
        Ok(self.conn.query_row("SELECT COUNT(*) FROM presets", [], |r| r.get(0))?)
    }

    pub fn insert_preset(&self, p: &Preset) -> AppResult<()> {
        self.conn.execute(
            "INSERT INTO presets (id, name, category, spec, favorite, builtin, created_at, last_used_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                p.id,
                p.name,
                p.category.as_str(),
                encode_spec(&p.spec)?,
                p.favorite as i64,
                p.builtin as i64,
                p.created_at,
                p.last_used_at
            ],
        )?;
        Ok(())
    }

    /// Inserts the preset unless a row with the same id exists.
    pub fn insert_preset_if_missing(&self, p: &Preset) -> AppResult<bool> {
        if self.get_preset(&p.id)?.is_some() {
            return Ok(false);
        }
        self.insert_preset(p)?;
        Ok(true)
    }

    pub fn update_preset(&self, p: &Preset) -> AppResult<bool> {
        let n = self.conn.execute(
            "UPDATE presets SET name = ?2, category = ?3, spec = ?4, favorite = ?5 WHERE id = ?1",
            params![p.id, p.name, p.category.as_str(), encode_spec(&p.spec)?, p.favorite as i64],
        )?;
        Ok(n > 0)
    }

    pub fn set_preset_favorite(&self, id: &str, favorite: bool) -> AppResult<bool> {
        let n = self
            .conn
            .execute("UPDATE presets SET favorite = ?2 WHERE id = ?1", params![id, favorite as i64])?;
        Ok(n > 0)
    }

    pub fn touch_preset(&self, id: &str, at: i64) -> AppResult<()> {
        self.conn
            .execute("UPDATE presets SET last_used_at = ?2 WHERE id = ?1", params![id, at])?;
        Ok(())
    }

    pub fn delete_preset(&self, id: &str) -> AppResult<bool> {
        Ok(self.conn.execute("DELETE FROM presets WHERE id = ?1", [id])? > 0)
    }

    // ── shortcuts ─────────────────────────────────────────────────────────

    pub fn list_shortcut_rows(&self) -> AppResult<Vec<(String, String, bool)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT action, accelerator, enabled FROM shortcuts ORDER BY action")?;
        let rows = stmt.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)? != 0))
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn upsert_shortcut(&self, action: &str, accelerator: &str, enabled: bool) -> AppResult<()> {
        self.conn.execute(
            "INSERT INTO shortcuts (action, accelerator, enabled) VALUES (?1, ?2, ?3)
             ON CONFLICT(action) DO UPDATE SET accelerator = excluded.accelerator, enabled = excluded.enabled",
            params![action, accelerator, enabled as i64],
        )?;
        Ok(())
    }

    // ── recents ───────────────────────────────────────────────────────────

    pub fn push_recent(&self, name: &str, source: &str, source_id: Option<&str>, spec: &CrosshairSpec, at: i64) -> AppResult<()> {
        let json = encode_spec(spec)?;
        // Re-applying the same crosshair moves it to the top instead of duplicating it.
        self.conn
            .execute("DELETE FROM recents WHERE spec = ?1", [&json])?;
        self.conn.execute(
            "INSERT INTO recents (name, source, source_id, spec, used_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![name, source, source_id, json, at],
        )?;
        self.conn.execute(
            "DELETE FROM recents WHERE id NOT IN (SELECT id FROM recents ORDER BY used_at DESC, id DESC LIMIT ?1)",
            [RECENTS_KEPT],
        )?;
        Ok(())
    }

    pub fn list_recents(&self, limit: i64) -> AppResult<Vec<RecentEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, source, source_id, spec, used_at FROM recents
             ORDER BY used_at DESC, id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map([limit], |r| {
            let spec: String = r.get(4)?;
            Ok(RecentEntry {
                id: r.get(0)?,
                name: r.get(1)?,
                source: r.get(2)?,
                source_id: r.get(3)?,
                spec: decode_spec(&spec),
                used_at: r.get(5)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    // ── maintenance ───────────────────────────────────────────────────────

    /// Deletes every row (schema is kept). Runs in a single transaction.
    pub fn reset_all(&mut self) -> AppResult<()> {
        let tx = self.conn.transaction()?;
        tx.execute_batch(
            "DELETE FROM settings; DELETE FROM app_state; DELETE FROM profiles;
             DELETE FROM presets; DELETE FROM shortcuts; DELETE FROM recents;",
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Runs `f` inside a transaction (all-or-nothing imports).
    pub fn transaction<T>(&mut self, f: impl FnOnce(&Storage) -> AppResult<T>) -> AppResult<T> {
        self.conn.execute_batch("BEGIN IMMEDIATE")?;
        match f(self) {
            Ok(v) => {
                self.conn.execute_batch("COMMIT")?;
                Ok(v)
            }
            Err(e) => {
                let _ = self.conn.execute_batch("ROLLBACK");
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crosshair::presets::PresetCategory;

    fn profile(id: &str, name: &str, sort: i64) -> Profile {
        Profile {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            spec: CrosshairSpec::default(),
            sort_index: sort,
            created_at: 1,
            updated_at: 1,
        }
    }

    #[test]
    fn migrations_are_applied_once() {
        let dir = std::env::temp_dir().join(format!("cs-test-{}", crate::util::new_id()));
        let path = dir.join(DB_FILE_NAME);
        {
            let s = Storage::open(&path).unwrap();
            assert_eq!(s.schema_version().unwrap(), migrations::latest_version());
            s.insert_profile(&profile("a", "A", 0)).unwrap();
        }
        // Re-opening keeps the data and does not re-run v1 (which would fail).
        let s = Storage::open(&path).unwrap();
        assert_eq!(s.count_profiles().unwrap(), 1);
        drop(s);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn newer_schema_is_refused() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "user_version", 999).unwrap();
        assert!(migrations::run(&mut conn).is_err());
    }

    #[test]
    fn profile_round_trip() {
        let s = Storage::open_in_memory().unwrap();
        let mut p = profile("p1", "Main", 0);
        p.spec.crosshair.color = "#123456".into();
        s.insert_profile(&p).unwrap();
        assert_eq!(s.get_profile("p1").unwrap().unwrap(), p);

        p.name = "Renamed".into();
        assert!(s.update_profile(&p).unwrap());
        assert_eq!(s.get_profile("p1").unwrap().unwrap().name, "Renamed");

        let mut spec = p.spec.clone();
        spec.crosshair.gap = 9.0;
        assert!(s.update_profile_spec("p1", &spec, 5).unwrap());
        let stored = s.get_profile("p1").unwrap().unwrap();
        assert_eq!(stored.spec.crosshair.gap, 9.0);
        assert_eq!(stored.updated_at, 5);

        assert!(s.delete_profile("p1").unwrap());
        assert!(!s.delete_profile("p1").unwrap());
    }

    #[test]
    fn profiles_are_sorted() {
        let s = Storage::open_in_memory().unwrap();
        s.insert_profile(&profile("b", "B", 1)).unwrap();
        s.insert_profile(&profile("a", "A", 0)).unwrap();
        let ids: Vec<_> = s.list_profiles().unwrap().into_iter().map(|p| p.id).collect();
        assert_eq!(ids, ["a", "b"]);
        assert_eq!(s.next_profile_sort_index().unwrap(), 2);
    }

    #[test]
    fn corrupted_spec_falls_back_to_default() {
        let s = Storage::open_in_memory().unwrap();
        s.conn
            .execute(
                "INSERT INTO profiles (id, name, spec, created_at, updated_at) VALUES ('x', 'X', '{not json', 0, 0)",
                [],
            )
            .unwrap();
        assert_eq!(s.get_profile("x").unwrap().unwrap().spec, CrosshairSpec::default());
    }

    #[test]
    fn presets_and_favorites() {
        let s = Storage::open_in_memory().unwrap();
        let p = Preset {
            id: "pr".into(),
            name: "Ring".into(),
            category: PresetCategory::Circle,
            spec: CrosshairSpec::default(),
            favorite: false,
            builtin: true,
            created_at: 0,
            last_used_at: None,
        };
        assert!(s.insert_preset_if_missing(&p).unwrap());
        assert!(!s.insert_preset_if_missing(&p).unwrap());
        assert!(s.set_preset_favorite("pr", true).unwrap());
        s.touch_preset("pr", 42).unwrap();
        let got = s.get_preset("pr").unwrap().unwrap();
        assert!(got.favorite);
        assert_eq!(got.last_used_at, Some(42));
        assert_eq!(got.category, PresetCategory::Circle);
    }

    #[test]
    fn recents_deduplicate_and_are_capped() {
        let s = Storage::open_in_memory().unwrap();
        let mut spec = CrosshairSpec::default();
        for i in 0..(RECENTS_KEPT + 10) {
            spec.crosshair.gap = (i % 100) as f32;
            s.push_recent(&format!("n{i}"), "preset", None, &spec, i).unwrap();
        }
        assert_eq!(s.list_recents(100).unwrap().len() as i64, RECENTS_KEPT);
        // Same spec again: moved to the top, not duplicated.
        s.push_recent("again", "preset", None, &spec, 10_000).unwrap();
        let list = s.list_recents(100).unwrap();
        assert_eq!(list[0].name, "again");
        assert_eq!(list.iter().filter(|r| r.spec == spec).count(), 1);
    }

    #[test]
    fn transaction_rolls_back_on_error() {
        let mut s = Storage::open_in_memory().unwrap();
        let res: AppResult<()> = s.transaction(|tx| {
            tx.insert_profile(&profile("t", "T", 0))?;
            Err(crate::error::AppError::validation("boom"))
        });
        assert!(res.is_err());
        assert_eq!(s.count_profiles().unwrap(), 0);
    }

    #[test]
    fn reset_clears_everything() {
        let mut s = Storage::open_in_memory().unwrap();
        s.insert_profile(&profile("a", "A", 0)).unwrap();
        s.set_value("k", "v").unwrap();
        s.save_settings_json(1, "{}").unwrap();
        s.reset_all().unwrap();
        assert_eq!(s.count_profiles().unwrap(), 0);
        assert!(s.get_value("k").unwrap().is_none());
        assert!(s.load_settings_json().unwrap().is_none());
    }
}
