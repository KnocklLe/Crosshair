use tauri::{AppHandle, State};

use crate::core::AppCore;
use crate::error::AppResult;
use crate::events::emit_state;
use crate::overlay::OverlayStatus;

#[tauri::command]
pub async fn get_overlay_status(core: State<'_, AppCore>) -> AppResult<OverlayStatus> {
    Ok(core.overlay.status())
}

/// Returns the state confirmed by the overlay thread.
#[tauri::command]
pub async fn set_overlay_visible(app: AppHandle, core: State<'_, AppCore>, visible: bool) -> AppResult<OverlayStatus> {
    let status = core.set_overlay_visible(visible)?;
    emit_state(&app, "ui");
    Ok(status)
}

#[tauri::command]
pub async fn toggle_overlay(app: AppHandle, core: State<'_, AppCore>) -> AppResult<OverlayStatus> {
    let status = core.toggle_overlay()?;
    emit_state(&app, "ui");
    Ok(status)
}

/// Re-reads the monitors and redraws the overlay.
#[tauri::command]
pub async fn refresh_overlay(core: State<'_, AppCore>) -> AppResult<OverlayStatus> {
    core.overlay.refresh()
}
