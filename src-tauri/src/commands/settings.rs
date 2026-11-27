use tauri::{AppHandle, State};

use crate::core::AppCore;
use crate::error::AppResult;
use crate::events::emit_state;
use crate::profiles::transfer::ImportReport;
use crate::settings::Settings;

use super::files::{dated_file_name, pick_import, save_json};
use super::shortcuts::register_from_storage;

#[tauri::command]
pub async fn get_settings(core: State<'_, AppCore>) -> AppResult<Settings> {
    Ok(core.settings())
}

/// Validates, applies side effects (startup entry, overlay options) and saves.
/// Returns the settings as actually applied.
#[tauri::command]
pub async fn update_settings(app: AppHandle, core: State<'_, AppCore>, settings: Settings) -> AppResult<Settings> {
    let applied = core.update_settings(settings)?;
    emit_state(&app, "ui");
    Ok(applied)
}

/// Deletes every profile, custom preset, favourite, shortcut and setting.
#[tauri::command]
pub async fn reset_all_data(app: AppHandle, core: State<'_, AppCore>) -> AppResult<Settings> {
    core.reset_all()?;
    register_from_storage(&app);
    emit_state(&app, "ui");
    Ok(core.settings())
}

/// Full backup: profiles, custom presets, settings and shortcuts.
#[tauri::command]
pub async fn export_backup(app: AppHandle, core: State<'_, AppCore>, title: String) -> AppResult<Option<String>> {
    let json = core.export_backup()?;
    save_json(&app, &title, &dated_file_name("crosshair-studio-backup"), &json)
}

#[tauri::command]
pub async fn import_backup(
    app: AppHandle,
    core: State<'_, AppCore>,
    title: String,
    restore_settings: bool,
) -> AppResult<Option<ImportReport>> {
    let Some(bundle) = pick_import(&app, &title)? else { return Ok(None) };
    let report = core.import_bundle(bundle, restore_settings)?;
    if restore_settings {
        register_from_storage(&app);
    }
    emit_state(&app, "ui");
    Ok(Some(report))
}
