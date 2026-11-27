use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager, State};

use crate::core::AppCore;
use crate::error::{AppError, AppResult};
use crate::events::{emit_state, emit_warning, EVT_SHORTCUTS};
use crate::shortcuts::{
    default_bindings, find_conflict, manager, normalize_accelerator, ShortcutAction, ShortcutBinding, ShortcutStatus,
};

/// Registration statuses as reported by the OS.
#[derive(Default)]
pub struct ShortcutRegistry(pub Mutex<Vec<ShortcutStatus>>);

impl ShortcutRegistry {
    pub fn list(&self) -> Vec<ShortcutStatus> {
        self.0.lock().map(|v| v.clone()).unwrap_or_default()
    }

    fn set(&self, statuses: Vec<ShortcutStatus>) {
        if let Ok(mut v) = self.0.lock() {
            *v = statuses;
        }
    }

    fn put(&self, status: ShortcutStatus) {
        if let Ok(mut v) = self.0.lock() {
            match v.iter_mut().find(|s| s.action == status.action) {
                Some(slot) => *slot = status,
                None => v.push(status),
            }
        }
    }
}

fn action_label(a: ShortcutAction) -> &'static str {
    match a {
        ShortcutAction::ToggleOverlay => "Show / hide the crosshair",
        ShortcutAction::ToggleDot => "Toggle the center dot",
        ShortcutAction::NextProfile => "Next profile",
    }
}

/// Runs an action triggered by a global shortcut or the tray menu.
pub fn run_action(app: &AppHandle, action: ShortcutAction, source: &str) {
    let Some(core) = app.try_state::<AppCore>() else { return };
    let result = match action {
        ShortcutAction::ToggleOverlay => core.toggle_overlay().map(|_| ()),
        ShortcutAction::ToggleDot => core.toggle_dot_mode().map(|_| ()),
        ShortcutAction::NextProfile => core.next_profile().map(|_| ()),
    };
    match result {
        Ok(()) => emit_state(app, source),
        Err(e) => emit_warning(app, "action_failed", e.to_string()),
    }
}

/// Hotkey callback (plugin thread): the work runs on a worker thread.
fn on_hotkey(app: &AppHandle, action: ShortcutAction) {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || run_action(&app, action, "shortcut"));
}

/// (Re)registers every stored binding and records the real results.
pub fn register_from_storage(app: &AppHandle) {
    let Some(core) = app.try_state::<AppCore>() else { return };
    let bindings = core.shortcut_bindings().unwrap_or_else(|_| default_bindings());
    let statuses = manager::register_all(app, &bindings, on_hotkey);
    for s in statuses.iter().filter(|s| s.enabled && !s.registered) {
        emit_warning(app, "shortcut_unavailable", s.error.clone().unwrap_or_default());
    }
    if let Some(registry) = app.try_state::<ShortcutRegistry>() {
        registry.set(statuses.clone());
    }
    let _ = app.emit(EVT_SHORTCUTS, statuses);
}

#[tauri::command]
pub async fn list_shortcuts(registry: State<'_, ShortcutRegistry>) -> AppResult<Vec<ShortcutStatus>> {
    Ok(registry.list())
}

/// Changes one shortcut. On OS refusal the previous shortcut is restored and
/// the reason is returned.
#[tauri::command]
pub async fn set_shortcut(
    app: AppHandle,
    core: State<'_, AppCore>,
    registry: State<'_, ShortcutRegistry>,
    action: ShortcutAction,
    accelerator: String,
    enabled: bool,
) -> AppResult<Vec<ShortcutStatus>> {
    let accelerator = normalize_accelerator(&accelerator)?;
    let bindings = core.shortcut_bindings()?;
    if enabled {
        if let Some(other) = find_conflict(&bindings, action, &accelerator) {
            return Err(AppError::Shortcut(format!(
                "{accelerator} is already assigned to \"{}\".",
                action_label(other)
            )));
        }
    }
    let previous = bindings.iter().find(|b| b.action == action).cloned();
    if let Some(prev) = &previous {
        manager::unregister(&app, &prev.accelerator);
    }
    let binding = ShortcutBinding { action, accelerator, enabled };
    let status = manager::register(&app, &binding, on_hotkey);
    if enabled && !status.registered {
        if let Some(prev) = &previous {
            registry.put(manager::register(&app, prev, on_hotkey));
        }
        return Err(AppError::Shortcut(status.error.unwrap_or_else(|| "Registration failed.".into())));
    }
    core.save_shortcut(&binding)?;
    registry.put(status);
    let list = registry.list();
    let _ = app.emit(EVT_SHORTCUTS, list.clone());
    Ok(list)
}

#[tauri::command]
pub async fn reset_shortcuts(app: AppHandle, core: State<'_, AppCore>, registry: State<'_, ShortcutRegistry>) -> AppResult<Vec<ShortcutStatus>> {
    for b in default_bindings() {
        core.save_shortcut(&b)?;
    }
    register_from_storage(&app);
    Ok(registry.list())
}
