use tauri::ipc::Response;
use tauri::{AppHandle, State};

use crate::core::AppCore;
use crate::crosshair::render::{render, to_rgba_straight};
use crate::crosshair::validate::validate_spec;
use crate::crosshair::CrosshairSpec;
use crate::error::{AppError, AppResult};
use crate::events::emit_state;
use crate::overlay::OverlayStatus;
use crate::profiles::Profile;

/// Renders a spec with the overlay renderer. Binary response:
/// `[width u32 LE][height u32 LE][RGBA bytes…]` (straight alpha).
#[tauri::command]
pub fn render_preview(spec: CrosshairSpec, scale: Option<f32>) -> AppResult<Response> {
    validate_spec(&spec)?;
    let scale = scale.unwrap_or(1.0);
    if !scale.is_finite() || !(0.5..=4.0).contains(&scale) {
        return Err(AppError::validation("scale must be between 0.5 and 4"));
    }
    let pixmap = render(&spec, scale);
    let (w, h) = (pixmap.width(), pixmap.height());
    let mut out = Vec::with_capacity(8 + (w * h * 4) as usize);
    out.extend_from_slice(&w.to_le_bytes());
    out.extend_from_slice(&h.to_le_bytes());
    out.extend_from_slice(&to_rgba_straight(pixmap));
    Ok(Response::new(out))
}

/// Live edit of the active profile (overlay updated immediately).
#[tauri::command]
pub async fn set_active_spec(
    app: AppHandle,
    core: State<'_, AppCore>,
    spec: CrosshairSpec,
    persist: bool,
) -> AppResult<OverlayStatus> {
    let status = core.set_active_spec(spec, persist)?;
    if persist {
        emit_state(&app, "ui");
    }
    Ok(status)
}

#[tauri::command]
pub async fn toggle_dot_mode(app: AppHandle, core: State<'_, AppCore>) -> AppResult<Profile> {
    let p = core.toggle_dot_mode()?;
    emit_state(&app, "ui");
    Ok(p)
}
