//! Application services: the business rules shared by the Tauri commands,
//! the global shortcuts and the tray menu.
//!
//! Locking rule: a lock is never held while waiting for the overlay thread,
//! so overlay callbacks can always read the state.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use std::time::Instant;

use serde::Serialize;

use crate::crosshair::presets::{builtin_presets, PresetCategory};
use crate::crosshair::validate::{clean_description, clean_name, validate_spec, LIMITS};
use crate::crosshair::{CrosshairSpec, SpecKind};
use crate::error::{AppError, AppResult};
use crate::library::{Preset, PresetInput, RecentEntry};
use crate::overlay::{EventSink, OverlayEngine, OverlayOptions, OverlayStatus};
use crate::profiles::transfer::{ExportFile, ImportBundle, ImportReport};
use crate::profiles::{Profile, ProfileInput};
use crate::settings::{autostart, Settings, SETTINGS_VERSION};
use crate::shortcuts::{default_bindings, ShortcutAction, ShortcutBinding};
use crate::storage::{Storage, DB_FILE_NAME};
use crate::util::{new_id, now_ms, unique_name};

const KEY_ACTIVE_PROFILE: &str = "active_profile_id";
const KEY_PRESETS_SEEDED: &str = "builtin_presets_seeded";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

/// Snapshot pushed to the interface after every state change.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StateSnapshot {
    pub overlay: OverlayStatus,
    pub active_profile: Profile,
    pub profile_count: i64,
    /// What triggered the change: "ui", "shortcut", "tray", "startup".
    pub source: String,
}

struct Runtime {
    settings: Settings,
    active: Profile,
}

pub struct AppCore {
    storage: Mutex<Storage>,
    runtime: Mutex<Runtime>,
    pub overlay: OverlayEngine,
    data_dir: PathBuf,
    pub started_at: Instant,
    pub launched_at_startup: bool,
}

/// Seeds the database on first launch (idempotent).
fn ensure_defaults(storage: &Storage) -> AppResult<()> {
    if storage.get_value(KEY_PRESETS_SEEDED)?.is_none() {
        seed_builtin_presets(storage)?;
        storage.set_value(KEY_PRESETS_SEEDED, "1")?;
    }
    let existing: Vec<String> = storage.list_shortcut_rows()?.into_iter().map(|(a, _, _)| a).collect();
    for b in default_bindings() {
        if !existing.iter().any(|a| a == b.action.as_str()) {
            storage.upsert_shortcut(b.action.as_str(), &b.accelerator, b.enabled)?;
        }
    }
    if storage.count_profiles()? == 0 {
        let now = now_ms();
        storage.insert_profile(&Profile {
            id: new_id(),
            name: "Default".into(),
            description: String::new(),
            spec: CrosshairSpec::default(),
            sort_index: 0,
            created_at: now,
            updated_at: now,
        })?;
    }
    Ok(())
}

fn seed_builtin_presets(storage: &Storage) -> AppResult<usize> {
    let now = now_ms();
    let mut added = 0;
    for (i, p) in builtin_presets().into_iter().enumerate() {
        let preset = Preset {
            id: p.id.into(),
            name: p.name.into(),
            category: p.category,
            spec: p.spec,
            favorite: false,
            builtin: true,
            // Keeps the declaration order in the library.
            created_at: now + i as i64,
            last_used_at: None,
        };
        if storage.insert_preset_if_missing(&preset)? {
            added += 1;
        }
    }
    Ok(added)
}

fn resolve_active(storage: &Storage) -> AppResult<Profile> {
    if let Some(id) = storage.get_value(KEY_ACTIVE_PROFILE)? {
        if let Some(p) = storage.get_profile(&id)? {
            return Ok(p);
        }
    }
    let first = storage
        .list_profiles()?
        .into_iter()
        .next()
        .ok_or_else(|| AppError::not_found("No profile available"))?;
    storage.set_value(KEY_ACTIVE_PROFILE, &first.id)?;
    Ok(first)
}

fn load_settings(storage: &Storage) -> AppResult<Settings> {
    let mut settings = Settings::from_stored(storage.load_settings_json()?.as_deref());
    // The registry is the source of truth for "launch at startup".
    settings.launch_at_startup = autostart::is_enabled();
    save_settings(storage, &settings)?;
    Ok(settings)
}

fn save_settings(storage: &Storage, s: &Settings) -> AppResult<()> {
    storage.save_settings_json(SETTINGS_VERSION, &serde_json::to_string(s)?)
}

impl AppCore {
    pub fn init(data_dir: PathBuf, sink: EventSink) -> AppResult<Self> {
        let storage = Storage::open(&data_dir.join(DB_FILE_NAME))?;
        ensure_defaults(&storage)?;
        let settings = load_settings(&storage)?;
        let active = resolve_active(&storage)?;
        let overlay = OverlayEngine::start(
            active.spec.clone(),
            OverlayOptions::from_settings(&settings.overlay, settings.performance.lite_mode),
            settings.overlay.auto_activate,
            sink,
        )?;
        Ok(Self {
            storage: Mutex::new(storage),
            runtime: Mutex::new(Runtime { settings, active }),
            overlay,
            data_dir,
            started_at: Instant::now(),
            launched_at_startup: autostart::launched_at_startup(),
        })
    }

    // ── read access ───────────────────────────────────────────────────────

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn db_path(&self) -> PathBuf {
        self.data_dir.join(DB_FILE_NAME)
    }

    pub fn settings(&self) -> Settings {
        lock(&self.runtime).settings.clone()
    }

    pub fn active_profile(&self) -> Profile {
        lock(&self.runtime).active.clone()
    }

    pub fn snapshot(&self, source: &str) -> AppResult<StateSnapshot> {
        let profile_count = lock(&self.storage).count_profiles()?;
        Ok(StateSnapshot {
            overlay: self.overlay.status(),
            active_profile: self.active_profile(),
            profile_count,
            source: source.into(),
        })
    }

    pub fn with_storage<T>(&self, f: impl FnOnce(&Storage) -> AppResult<T>) -> AppResult<T> {
        f(&lock(&self.storage))
    }

    // ── overlay ───────────────────────────────────────────────────────────

    pub fn set_overlay_visible(&self, visible: bool) -> AppResult<OverlayStatus> {
        self.overlay.set_visible(visible)
    }

    pub fn toggle_overlay(&self) -> AppResult<OverlayStatus> {
        let requested = self.overlay.status().requested;
        self.overlay.set_visible(!requested)
    }

    /// Live edit of the active profile. `persist = false` during a drag,
    /// `true` once the value is settled.
    pub fn set_active_spec(&self, spec: CrosshairSpec, persist: bool) -> AppResult<OverlayStatus> {
        validate_spec(&spec)?;
        let id = {
            let mut rt = lock(&self.runtime);
            rt.active.spec = spec.clone();
            if persist {
                rt.active.updated_at = now_ms();
            }
            rt.active.id.clone()
        };
        if persist {
            lock(&self.storage).update_profile_spec(&id, &spec, now_ms())?;
        }
        self.overlay.set_spec(spec)
    }

    /// Ctrl+Shift+D: switches the active profile between Center Dot and crosshair.
    pub fn toggle_dot_mode(&self) -> AppResult<Profile> {
        let mut spec = self.active_profile().spec;
        spec.kind = match spec.kind {
            SpecKind::Dot => SpecKind::Crosshair,
            SpecKind::Crosshair => SpecKind::Dot,
        };
        self.set_active_spec(spec, true)?;
        Ok(self.active_profile())
    }

    // ── profiles ──────────────────────────────────────────────────────────

    pub fn list_profiles(&self) -> AppResult<Vec<Profile>> {
        lock(&self.storage).list_profiles()
    }

    pub fn create_profile(&self, input: ProfileInput) -> AppResult<Profile> {
        let name = clean_name(&input.name)?;
        let description = clean_description(&input.description)?;
        let spec = input.spec.unwrap_or_else(|| self.active_profile().spec);
        validate_spec(&spec)?;
        let storage = lock(&self.storage);
        let now = now_ms();
        let existing = storage.list_profiles()?;
        let profile = Profile {
            id: new_id(),
            name: unique_name(&name, existing.iter().map(|p| p.name.as_str()), LIMITS.name_max_chars),
            description,
            spec,
            sort_index: storage.next_profile_sort_index()?,
            created_at: now,
            updated_at: now,
        };
        storage.insert_profile(&profile)?;
        Ok(profile)
    }

    pub fn update_profile(&self, id: &str, input: ProfileInput) -> AppResult<Profile> {
        let name = clean_name(&input.name)?;
        let description = clean_description(&input.description)?;
        if let Some(spec) = &input.spec {
            validate_spec(spec)?;
        }
        let updated = {
            let storage = lock(&self.storage);
            let mut p = storage.get_profile(id)?.ok_or_else(|| AppError::not_found("Profile not found"))?;
            let others = storage.list_profiles()?;
            p.name = unique_name(&name, others.iter().filter(|o| o.id != id).map(|o| o.name.as_str()), LIMITS.name_max_chars);
            p.description = description;
            if let Some(spec) = input.spec {
                p.spec = spec;
            }
            p.updated_at = now_ms();
            storage.update_profile(&p)?;
            p
        };
        let is_active = {
            let mut rt = lock(&self.runtime);
            if rt.active.id == id {
                rt.active = updated.clone();
                true
            } else {
                false
            }
        };
        if is_active {
            self.overlay.set_spec(updated.spec.clone())?;
        }
        Ok(updated)
    }

    pub fn duplicate_profile(&self, id: &str) -> AppResult<Profile> {
        let source = lock(&self.storage).get_profile(id)?.ok_or_else(|| AppError::not_found("Profile not found"))?;
        // The active profile may hold unsaved live edits: duplicate what is displayed.
        let spec = if self.active_profile().id == id { self.active_profile().spec } else { source.spec };
        self.create_profile(ProfileInput {
            name: format!("{} copy", source.name.chars().take(LIMITS.name_max_chars - 5).collect::<String>()),
            description: source.description,
            spec: Some(spec),
        })
    }

    pub fn delete_profile(&self, id: &str) -> AppResult<()> {
        let next_active = {
            let storage = lock(&self.storage);
            let profiles = storage.list_profiles()?;
            if !profiles.iter().any(|p| p.id == id) {
                return Err(AppError::not_found("Profile not found"));
            }
            if profiles.len() <= 1 {
                return Err(AppError::validation("The last profile cannot be deleted."));
            }
            storage.delete_profile(id)?;
            let active_id = lock(&self.runtime).active.id.clone();
            (active_id == id).then(|| profiles.into_iter().find(|p| p.id != id)).flatten()
        };
        if let Some(next) = next_active {
            self.activate_profile(&next.id)?;
        }
        Ok(())
    }

    pub fn activate_profile(&self, id: &str) -> AppResult<Profile> {
        let profile = {
            let storage = lock(&self.storage);
            let p = storage.get_profile(id)?.ok_or_else(|| AppError::not_found("Profile not found"))?;
            storage.set_value(KEY_ACTIVE_PROFILE, &p.id)?;
            storage.push_recent(&p.name, "profile", Some(&p.id), &p.spec, now_ms())?;
            p
        };
        lock(&self.runtime).active = profile.clone();
        self.overlay.set_spec(profile.spec.clone())?;
        Ok(profile)
    }

    /// Ctrl+Shift+P: activates the next profile (wraps around).
    pub fn next_profile(&self) -> AppResult<Profile> {
        let profiles = self.list_profiles()?;
        let current = self.active_profile().id;
        let index = profiles.iter().position(|p| p.id == current).unwrap_or(0);
        let next = &profiles[(index + 1) % profiles.len()];
        self.activate_profile(&next.id.clone())
    }

    // ── library ───────────────────────────────────────────────────────────

    pub fn list_presets(&self) -> AppResult<Vec<Preset>> {
        lock(&self.storage).list_presets()
    }

    pub fn apply_preset(&self, id: &str) -> AppResult<Profile> {
        let preset = {
            let storage = lock(&self.storage);
            let p = storage.get_preset(id)?.ok_or_else(|| AppError::not_found("Preset not found"))?;
            let now = now_ms();
            storage.touch_preset(id, now)?;
            storage.push_recent(&p.name, "preset", Some(&p.id), &p.spec, now)?;
            p
        };
        self.set_active_spec(preset.spec, true)?;
        Ok(self.active_profile())
    }

    /// Applies a crosshair from the history.
    pub fn apply_recent(&self, recent_id: i64) -> AppResult<Profile> {
        let entry = lock(&self.storage)
            .list_recents(crate::library::RECENTS_KEPT)?
            .into_iter()
            .find(|r| r.id == recent_id)
            .ok_or_else(|| AppError::not_found("History entry not found"))?;
        self.set_active_spec(entry.spec.clone(), true)?;
        lock(&self.storage).push_recent(&entry.name, &entry.source, entry.source_id.as_deref(), &entry.spec, now_ms())?;
        Ok(self.active_profile())
    }

    pub fn create_preset(&self, input: PresetInput) -> AppResult<Preset> {
        let name = clean_name(&input.name)?;
        validate_spec(&input.spec)?;
        let storage = lock(&self.storage);
        let existing = storage.list_presets()?;
        let preset = Preset {
            id: new_id(),
            name: unique_name(&name, existing.iter().map(|p| p.name.as_str()), LIMITS.name_max_chars),
            category: input.category,
            spec: input.spec,
            favorite: false,
            builtin: false,
            created_at: now_ms(),
            last_used_at: None,
        };
        storage.insert_preset(&preset)?;
        Ok(preset)
    }

    pub fn update_preset(&self, id: &str, input: PresetInput) -> AppResult<Preset> {
        let name = clean_name(&input.name)?;
        validate_spec(&input.spec)?;
        let storage = lock(&self.storage);
        let mut p = storage.get_preset(id)?.ok_or_else(|| AppError::not_found("Preset not found"))?;
        if p.builtin {
            return Err(AppError::validation("Built-in presets are read-only: duplicate it to modify a copy."));
        }
        let others = storage.list_presets()?;
        p.name = unique_name(&name, others.iter().filter(|o| o.id != id).map(|o| o.name.as_str()), LIMITS.name_max_chars);
        p.category = input.category;
        p.spec = input.spec;
        storage.update_preset(&p)?;
        Ok(p)
    }

    pub fn duplicate_preset(&self, id: &str) -> AppResult<Preset> {
        let source = lock(&self.storage).get_preset(id)?.ok_or_else(|| AppError::not_found("Preset not found"))?;
        self.create_preset(PresetInput {
            name: format!("{} copy", source.name.chars().take(LIMITS.name_max_chars - 5).collect::<String>()),
            category: if source.builtin { PresetCategory::Custom } else { source.category },
            spec: source.spec,
        })
    }

    pub fn delete_preset(&self, id: &str) -> AppResult<()> {
        if !lock(&self.storage).delete_preset(id)? {
            return Err(AppError::not_found("Preset not found"));
        }
        Ok(())
    }

    pub fn set_preset_favorite(&self, id: &str, favorite: bool) -> AppResult<Preset> {
        let storage = lock(&self.storage);
        if !storage.set_preset_favorite(id, favorite)? {
            return Err(AppError::not_found("Preset not found"));
        }
        storage.get_preset(id)?.ok_or_else(|| AppError::not_found("Preset not found"))
    }

    /// Re-adds deleted built-in presets. Returns how many were restored.
    pub fn restore_builtin_presets(&self) -> AppResult<usize> {
        seed_builtin_presets(&lock(&self.storage))
    }

    pub fn recents(&self, limit: i64) -> AppResult<Vec<RecentEntry>> {
        lock(&self.storage).list_recents(limit.clamp(1, crate::library::RECENTS_KEPT))
    }

    // ── settings ──────────────────────────────────────────────────────────

    pub fn update_settings(&self, new: Settings) -> AppResult<Settings> {
        let mut new = new.validated()?;
        let old = self.settings();
        if new.launch_at_startup != autostart::is_enabled() {
            autostart::set_enabled(new.launch_at_startup)?;
        }
        new.launch_at_startup = autostart::is_enabled();
        save_settings(&lock(&self.storage), &new)?;
        lock(&self.runtime).settings = new.clone();
        if old.overlay != new.overlay || old.performance != new.performance {
            self.overlay
                .set_options(OverlayOptions::from_settings(&new.overlay, new.performance.lite_mode))?;
        }
        Ok(new)
    }

    // ── shortcuts (storage side; OS registration is done by the caller) ──

    pub fn shortcut_bindings(&self) -> AppResult<Vec<ShortcutBinding>> {
        let rows = lock(&self.storage).list_shortcut_rows()?;
        Ok(ShortcutAction::ALL
            .into_iter()
            .map(|action| {
                rows.iter()
                    .find(|(a, _, _)| a == action.as_str())
                    .map(|(_, accel, enabled)| ShortcutBinding { action, accelerator: accel.clone(), enabled: *enabled })
                    .unwrap_or_else(|| ShortcutBinding::default_for(action))
            })
            .collect())
    }

    pub fn save_shortcut(&self, b: &ShortcutBinding) -> AppResult<()> {
        lock(&self.storage).upsert_shortcut(b.action.as_str(), &b.accelerator, b.enabled)
    }

    // ── import / export ───────────────────────────────────────────────────

    pub fn export_profiles(&self, ids: Option<Vec<String>>) -> AppResult<String> {
        let mut profiles = self.list_profiles()?;
        // Export what is displayed for the active profile (live edits included).
        let active = self.active_profile();
        for p in &mut profiles {
            if p.id == active.id {
                p.spec = active.spec.clone();
            }
        }
        if let Some(ids) = ids {
            profiles.retain(|p| ids.contains(&p.id));
        }
        if profiles.is_empty() {
            return Err(AppError::validation("No profile selected."));
        }
        ExportFile::new(now_ms(), APP_VERSION).with_profiles(&profiles).to_json()
    }

    pub fn export_backup(&self) -> AppResult<String> {
        let profiles = self.list_profiles()?;
        let presets: Vec<Preset> = self.list_presets()?.into_iter().filter(|p| !p.builtin).collect();
        let mut file = ExportFile::new(now_ms(), APP_VERSION).with_profiles(&profiles).with_presets(&presets);
        file.settings = Some(self.settings());
        file.shortcuts = Some(self.shortcut_bindings()?);
        file.to_json()
    }

    /// Imports a validated bundle in a single transaction (all or nothing).
    pub fn import_bundle(&self, bundle: ImportBundle, restore_settings: bool) -> AppResult<ImportReport> {
        let mut report = ImportReport { skipped: bundle.skipped.clone(), ..ImportReport::default() };
        {
            let mut storage = lock(&self.storage);
            storage.transaction(|s| {
                let mut names: Vec<String> = s.list_profiles()?.into_iter().map(|p| p.name).collect();
                let mut sort = s.next_profile_sort_index()?;
                let now = now_ms();
                for p in &bundle.profiles {
                    let name = unique_name(&p.name, names.iter().map(String::as_str), LIMITS.name_max_chars);
                    names.push(name.clone());
                    s.insert_profile(&Profile {
                        id: new_id(),
                        name,
                        description: p.description.clone(),
                        spec: p.spec.clone(),
                        sort_index: sort,
                        created_at: if p.created_at > 0 { p.created_at } else { now },
                        updated_at: now,
                    })?;
                    sort += 1;
                    report.profiles_imported += 1;
                }
                let mut preset_names: Vec<String> = s.list_presets()?.into_iter().map(|p| p.name).collect();
                for p in &bundle.presets {
                    let name = unique_name(&p.name, preset_names.iter().map(String::as_str), LIMITS.name_max_chars);
                    preset_names.push(name.clone());
                    s.insert_preset(&Preset {
                        id: new_id(),
                        name,
                        category: p.category,
                        spec: p.spec.clone(),
                        favorite: p.favorite,
                        builtin: false,
                        created_at: now,
                        last_used_at: None,
                    })?;
                    report.presets_imported += 1;
                }
                if restore_settings {
                    if let Some(bindings) = &bundle.shortcuts {
                        for b in bindings {
                            if let Ok(accel) = crate::shortcuts::normalize_accelerator(&b.accelerator) {
                                s.upsert_shortcut(b.action.as_str(), &accel, b.enabled)?;
                            }
                        }
                    }
                }
                Ok(())
            })?;
        }
        if restore_settings {
            if let Some(settings) = bundle.settings {
                self.update_settings(settings)?;
                report.settings_restored = true;
            }
        }
        Ok(report)
    }

    /// Deletes all user data and starts again from the defaults.
    pub fn reset_all(&self) -> AppResult<()> {
        if autostart::is_enabled() {
            autostart::set_enabled(false)?;
        }
        let (settings, active) = {
            let mut storage = lock(&self.storage);
            storage.reset_all()?;
            ensure_defaults(&storage)?;
            (load_settings(&storage)?, resolve_active(&storage)?)
        };
        {
            let mut rt = lock(&self.runtime);
            rt.settings = settings.clone();
            rt.active = active.clone();
        }
        self.overlay
            .set_options(OverlayOptions::from_settings(&settings.overlay, settings.performance.lite_mode))?;
        self.overlay.set_spec(active.spec)?;
        self.overlay.set_visible(settings.overlay.auto_activate)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    //! Service tests against a real (temporary) database and the real overlay.
    use super::*;
    use crate::crosshair::DotConfig;
    use crate::profiles::transfer::parse_import;

    struct TempCore {
        core: AppCore,
        dir: PathBuf,
    }

    impl Drop for TempCore {
        fn drop(&mut self) {
            self.core.overlay.shutdown();
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    fn core() -> TempCore {
        let dir = std::env::temp_dir().join(format!("cs-core-{}", new_id()));
        let core = AppCore::init(dir.clone(), Box::new(|_| {})).expect("core starts");
        // Tests must not leave the crosshair on screen.
        core.set_overlay_visible(false).unwrap();
        TempCore { core, dir }
    }

    #[test]
    fn first_launch_creates_defaults() {
        let t = core();
        let c = &t.core;
        assert_eq!(c.list_profiles().unwrap().len(), 1);
        assert!(c.list_presets().unwrap().len() >= 20);
        assert_eq!(c.shortcut_bindings().unwrap().len(), 3);
        assert_eq!(c.active_profile().name, "Default");
    }

    #[test]
    fn profile_lifecycle() {
        let t = core();
        let c = &t.core;
        let p = c.create_profile(ProfileInput { name: "Valorant".into(), description: "".into(), spec: None }).unwrap();
        let dup = c.create_profile(ProfileInput { name: "valorant".into(), description: "".into(), spec: None }).unwrap();
        assert_eq!(dup.name, "valorant (2)", "names are made unique");

        let copy = c.duplicate_profile(&p.id).unwrap();
        assert_eq!(copy.name, "Valorant copy");

        c.activate_profile(&p.id).unwrap();
        assert_eq!(c.active_profile().id, p.id);
        c.delete_profile(&p.id).unwrap();
        assert_ne!(c.active_profile().id, p.id, "deleting the active profile activates another one");

        for prof in c.list_profiles().unwrap().into_iter().skip(1) {
            c.delete_profile(&prof.id).unwrap();
        }
        let last = c.list_profiles().unwrap();
        assert_eq!(last.len(), 1);
        assert!(c.delete_profile(&last[0].id).is_err(), "the last profile is protected");
    }

    #[test]
    fn next_profile_wraps_around() {
        let t = core();
        let c = &t.core;
        let first = c.active_profile();
        let second = c.create_profile(ProfileInput { name: "B".into(), description: "".into(), spec: None }).unwrap();
        assert_eq!(c.next_profile().unwrap().id, second.id);
        assert_eq!(c.next_profile().unwrap().id, first.id);
    }

    #[test]
    fn live_edit_then_persist() {
        let t = core();
        let c = &t.core;
        let mut spec = c.active_profile().spec;
        spec.crosshair.gap = 11.0;
        c.set_active_spec(spec.clone(), false).unwrap();
        assert_eq!(c.active_profile().spec.crosshair.gap, 11.0);
        let stored = c.with_storage(|s| s.get_profile(&c.active_profile().id)).unwrap().unwrap();
        assert_ne!(stored.spec.crosshair.gap, 11.0, "not persisted during a drag");
        c.set_active_spec(spec, true).unwrap();
        let stored = c.with_storage(|s| s.get_profile(&c.active_profile().id)).unwrap().unwrap();
        assert_eq!(stored.spec.crosshair.gap, 11.0);

        let mut bad = c.active_profile().spec;
        bad.crosshair.thickness = 999.0;
        assert!(c.set_active_spec(bad, true).is_err());
    }

    #[test]
    fn toggle_dot_mode_keeps_both_configs() {
        let t = core();
        let c = &t.core;
        let mut spec = c.active_profile().spec;
        spec.dot = DotConfig { size: 9.0, ..DotConfig::default() };
        c.set_active_spec(spec, true).unwrap();
        assert_eq!(c.toggle_dot_mode().unwrap().spec.kind, SpecKind::Dot);
        let back = c.toggle_dot_mode().unwrap();
        assert_eq!(back.spec.kind, SpecKind::Crosshair);
        assert_eq!(back.spec.dot.size, 9.0);
    }

    #[test]
    fn presets_apply_favorite_duplicate_delete() {
        let t = core();
        let c = &t.core;
        let presets = c.list_presets().unwrap();
        let ring = presets.iter().find(|p| p.id == "builtin-ring").unwrap();
        c.apply_preset(&ring.id).unwrap();
        assert_eq!(c.active_profile().spec, ring.spec);
        assert_eq!(c.recents(5).unwrap()[0].name, "Ring");

        assert!(c.set_preset_favorite(&ring.id, true).unwrap().favorite);
        assert!(c
            .update_preset(&ring.id, PresetInput { name: "X".into(), category: PresetCategory::Dot, spec: ring.spec.clone() })
            .is_err());
        let copy = c.duplicate_preset(&ring.id).unwrap();
        assert!(!copy.builtin);
        let edited = c
            .update_preset(&copy.id, PresetInput { name: "My ring".into(), category: PresetCategory::Circle, spec: ring.spec.clone() })
            .unwrap();
        assert_eq!(edited.name, "My ring");
        c.delete_preset(&copy.id).unwrap();
        c.delete_preset(&ring.id).unwrap();
        assert_eq!(c.restore_builtin_presets().unwrap(), 1);
    }

    #[test]
    fn export_import_round_trip() {
        let t = core();
        let c = &t.core;
        c.create_profile(ProfileInput { name: "Exported".into(), description: "d".into(), spec: None }).unwrap();
        let json = c.export_profiles(None).unwrap();
        let bundle = parse_import(json.as_bytes()).unwrap();
        let report = c.import_bundle(bundle, false).unwrap();
        assert_eq!(report.profiles_imported, 2);
        let names: Vec<String> = c.list_profiles().unwrap().into_iter().map(|p| p.name).collect();
        assert!(names.contains(&"Exported (2)".to_string()), "{names:?}");

        let backup = c.export_backup().unwrap();
        let b = parse_import(backup.as_bytes()).unwrap();
        assert!(b.settings.is_some() && b.shortcuts.is_some());
    }

    #[test]
    fn reset_restores_defaults() {
        let t = core();
        let c = &t.core;
        c.create_profile(ProfileInput { name: "Temp".into(), description: "".into(), spec: None }).unwrap();
        c.reset_all().unwrap();
        c.set_overlay_visible(false).unwrap();
        assert_eq!(c.list_profiles().unwrap().len(), 1);
        assert_eq!(c.settings().theme, crate::settings::Theme::Dark);
    }
}
