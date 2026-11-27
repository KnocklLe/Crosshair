use tauri::{AppHandle, State};

use crate::core::AppCore;
use crate::error::AppResult;
use crate::events::emit_state;
use crate::library::{Preset, PresetInput};
use crate::profiles::Profile;

#[tauri::command]
pub async fn list_presets(core: State<'_, AppCore>) -> AppResult<Vec<Preset>> {
    core.list_presets()
}

/// Applies the preset to the active profile and the overlay.
#[tauri::command]
pub async fn apply_preset(app: AppHandle, core: State<'_, AppCore>, id: String) -> AppResult<Profile> {
    let p = core.apply_preset(&id)?;
    emit_state(&app, "ui");
    Ok(p)
}

#[tauri::command]
pub async fn create_preset(core: State<'_, AppCore>, input: PresetInput) -> AppResult<Preset> {
    core.create_preset(input)
}

#[tauri::command]
pub async fn update_preset(core: State<'_, AppCore>, id: String, input: PresetInput) -> AppResult<Preset> {
    core.update_preset(&id, input)
}

#[tauri::command]
pub async fn duplicate_preset(core: State<'_, AppCore>, id: String) -> AppResult<Preset> {
    core.duplicate_preset(&id)
}

#[tauri::command]
pub async fn delete_preset(core: State<'_, AppCore>, id: String) -> AppResult<()> {
    core.delete_preset(&id)
}

#[tauri::command]
pub async fn set_preset_favorite(core: State<'_, AppCore>, id: String, favorite: bool) -> AppResult<Preset> {
    core.set_preset_favorite(&id, favorite)
}

#[tauri::command]
pub async fn restore_builtin_presets(core: State<'_, AppCore>) -> AppResult<usize> {
    core.restore_builtin_presets()
}
