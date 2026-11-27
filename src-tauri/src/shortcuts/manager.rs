//! OS registration of the global shortcuts (RegisterHotKey through
//! `tauri-plugin-global-shortcut`). Each registration result is reported as
//! it really happened: a shortcut already taken by another application is
//! shown as not registered, with the reason.

use tauri::{AppHandle, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use super::{ShortcutAction, ShortcutBinding, ShortcutStatus};

/// Called on the plugin's thread when a registered shortcut is pressed.
pub type ActionHandler<R> = fn(&AppHandle<R>, ShortcutAction);

fn describe_error(accelerator: &str, e: impl std::fmt::Display) -> String {
    let raw = e.to_string();
    let lower = raw.to_lowercase();
    if lower.contains("already") || lower.contains("1409") {
        format!("{accelerator} is already used by another application. Choose another combination.")
    } else {
        format!("{accelerator} could not be registered: {raw}")
    }
}

/// Registers one binding. Returns the real status.
pub fn register<R: Runtime>(app: &AppHandle<R>, binding: &ShortcutBinding, handler: ActionHandler<R>) -> ShortcutStatus {
    let mut status = ShortcutStatus {
        action: binding.action,
        accelerator: binding.accelerator.clone(),
        enabled: binding.enabled,
        registered: false,
        error: None,
    };
    if !binding.enabled {
        return status;
    }
    let action = binding.action;
    let result = app.global_shortcut().on_shortcut(binding.accelerator.as_str(), move |app, _shortcut, event| {
        if event.state == ShortcutState::Pressed {
            handler(app, action);
        }
    });
    match result {
        Ok(()) => status.registered = true,
        Err(e) => status.error = Some(describe_error(&binding.accelerator, e)),
    }
    status
}

pub fn unregister<R: Runtime>(app: &AppHandle<R>, accelerator: &str) {
    let gs = app.global_shortcut();
    if gs.is_registered(accelerator) {
        let _ = gs.unregister(accelerator);
    }
}

pub fn register_all<R: Runtime>(app: &AppHandle<R>, bindings: &[ShortcutBinding], handler: ActionHandler<R>) -> Vec<ShortcutStatus> {
    let _ = app.global_shortcut().unregister_all();
    bindings.iter().map(|b| register(app, b, handler)).collect()
}
