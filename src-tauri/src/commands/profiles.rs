use tauri::{AppHandle, State};

use crate::core::AppCore;
use crate::error::AppResult;
use crate::events::emit_state;
use crate::profiles::transfer::ImportReport;
use crate::profiles::{Profile, ProfileInput};

use super::files::{dated_file_name, pick_import, save_json};

#[tauri::command]
pub async fn list_profiles(core: State<'_, AppCore>) -> AppResult<Vec<Profile>> {
    core.list_profiles()
}

#[tauri::command]
pub async fn create_profile(app: AppHandle, core: State<'_, AppCore>, input: ProfileInput) -> AppResult<Profile> {
    let p = core.create_profile(input)?;
    emit_state(&app, "ui");
    Ok(p)
}

#[tauri::command]
pub async fn update_profile(app: AppHandle, core: State<'_, AppCore>, id: String, input: ProfileInput) -> AppResult<Profile> {
    let p = core.update_profile(&id, input)?;
    emit_state(&app, "ui");
    Ok(p)
}

#[tauri::command]
pub async fn duplicate_profile(app: AppHandle, core: State<'_, AppCore>, id: String) -> AppResult<Profile> {
    let p = core.duplicate_profile(&id)?;
    emit_state(&app, "ui");
    Ok(p)
}

#[tauri::command]
pub async fn delete_profile(app: AppHandle, core: State<'_, AppCore>, id: String) -> AppResult<()> {
    core.delete_profile(&id)?;
    emit_state(&app, "ui");
    Ok(())
}

#[tauri::command]
pub async fn activate_profile(app: AppHandle, core: State<'_, AppCore>, id: String) -> AppResult<Profile> {
    let p = core.activate_profile(&id)?;
    emit_state(&app, "ui");
    Ok(p)
}

#[tauri::command]
pub async fn next_profile(app: AppHandle, core: State<'_, AppCore>) -> AppResult<Profile> {
    let p = core.next_profile()?;
    emit_state(&app, "ui");
    Ok(p)
}

/// Exports the given profiles (all when `ids` is absent). Returns the saved
/// path, or `None` if the user cancelled the dialog.
#[tauri::command]
pub async fn export_profiles(app: AppHandle, core: State<'_, AppCore>, ids: Option<Vec<String>>, title: String) -> AppResult<Option<String>> {
    let json = core.export_profiles(ids)?;
    save_json(&app, &title, &dated_file_name("crosshair-studio-profiles"), &json)
}

#[tauri::command]
pub async fn import_profiles(app: AppHandle, core: State<'_, AppCore>, title: String) -> AppResult<Option<ImportReport>> {
    let Some(bundle) = pick_import(&app, &title)? else { return Ok(None) };
    let report = core.import_bundle(bundle, false)?;
    emit_state(&app, "ui");
    Ok(Some(report))
}
