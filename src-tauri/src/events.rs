//! Events pushed from Rust to the interface.

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::core::AppCore;
use crate::overlay::OverlayEvent;
use crate::tray;

/// Full state snapshot (`StateSnapshot`).
pub const EVT_STATE: &str = "app://state";
/// Overlay status only (`OverlayStatus`), emitted by the overlay thread.
pub const EVT_OVERLAY: &str = "app://overlay";
/// Monitor configuration changed.
pub const EVT_DISPLAYS: &str = "app://displays";
/// Something the user must know (`Warning`).
pub const EVT_WARNING: &str = "app://warning";
/// Shortcut registration statuses changed.
pub const EVT_SHORTCUTS: &str = "app://shortcuts";

#[derive(Debug, Clone, Serialize)]
pub struct Warning {
    pub code: String,
    pub message: String,
}

/// Emits a full snapshot and refreshes the tray menu.
pub fn emit_state(app: &AppHandle, source: &str) {
    if let Some(core) = app.try_state::<AppCore>() {
        if let Ok(snapshot) = core.snapshot(source) {
            let _ = app.emit(EVT_STATE, snapshot);
        }
    }
    tray::schedule_refresh(app);
}

pub fn emit_warning(app: &AppHandle, code: &str, message: impl Into<String>) {
    let _ = app.emit(EVT_WARNING, Warning { code: code.into(), message: message.into() });
}

/// Called on the overlay thread: must never block.
pub fn on_overlay_event(app: &AppHandle, event: OverlayEvent) {
    match event {
        OverlayEvent::StatusChanged(status) => {
            let _ = app.emit(EVT_OVERLAY, status);
            tray::schedule_refresh(app);
        }
        OverlayEvent::DisplaysChanged => {
            let _ = app.emit(EVT_DISPLAYS, crate::displays::list());
        }
        OverlayEvent::Warning { code, message } => emit_warning(app, code, message),
    }
}
