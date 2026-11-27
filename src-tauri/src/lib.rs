//! Crosshair Studio — backend entry point.
//!
//! Module map:
//! * [`crosshair`] — model, validation, geometry, renderer, presets (pure)
//! * [`overlay`]   — native Win32 overlay window on its own thread
//! * [`displays`]  — monitor enumeration and placement
//! * [`profiles`], [`library`], [`settings`], [`shortcuts`] — domain models
//! * [`storage`]   — SQLite + migrations
//! * [`core`]      — services (business rules)
//! * [`commands`]  — Tauri commands (thin layer)

pub mod commands;
pub mod core;
pub mod crosshair;
pub mod diagnostics;
pub mod displays;
pub mod error;
pub mod events;
pub mod library;
pub mod overlay;
pub mod profiles;
pub mod settings;
pub mod shortcuts;
pub mod storage;
pub mod tray;
pub mod util;

use std::sync::Mutex;

use tauri::{Manager, RunEvent, WindowEvent};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

use crate::commands::shortcuts::{register_from_storage, ShortcutRegistry};
use crate::core::AppCore;
use crate::overlay::EventSink;

fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle().clone();
    app.manage(ShortcutRegistry::default());
    app.manage(diagnostics::Sampler::default());
    app.manage(tray::UiLanguage(Mutex::new("en".into())));

    let data_dir = app.path().app_data_dir()?;
    let sink_handle = handle.clone();
    let sink: EventSink = Box::new(move |event| events::on_overlay_event(&sink_handle, event));
    let core = match AppCore::init(data_dir, sink) {
        Ok(core) => core,
        Err(e) => {
            handle
                .dialog()
                .message(format!("Crosshair Studio could not start:\n\n{e}"))
                .title("Crosshair Studio")
                .kind(MessageDialogKind::Error)
                .blocking_show();
            return Err(Box::new(e));
        }
    };
    app.manage(core);

    register_from_storage(&handle);
    tray::create(&handle)?;
    Ok(())
}

pub fn run() {
    let app = tauri::Builder::default()
        // Must be first: a second launch focuses the running instance.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| tray::show_main_window(app)))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .setup(setup)
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    let to_tray = window.try_state::<AppCore>().map(|c| c.settings().close_to_tray).unwrap_or(false);
                    if to_tray {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::get_bootstrap,
            commands::app::app_ready,
            commands::app::set_ui_language,
            commands::app::get_dashboard,
            commands::app::get_performance,
            commands::app::open_data_folder,
            commands::app::quit_app,
            commands::app::list_recents,
            commands::app::apply_recent,
            commands::overlay::get_overlay_status,
            commands::overlay::set_overlay_visible,
            commands::overlay::toggle_overlay,
            commands::overlay::refresh_overlay,
            commands::crosshair::render_preview,
            commands::crosshair::set_active_spec,
            commands::crosshair::toggle_dot_mode,
            commands::profiles::list_profiles,
            commands::profiles::create_profile,
            commands::profiles::update_profile,
            commands::profiles::duplicate_profile,
            commands::profiles::delete_profile,
            commands::profiles::activate_profile,
            commands::profiles::next_profile,
            commands::profiles::export_profiles,
            commands::profiles::import_profiles,
            commands::presets::list_presets,
            commands::presets::apply_preset,
            commands::presets::create_preset,
            commands::presets::update_preset,
            commands::presets::duplicate_preset,
            commands::presets::delete_preset,
            commands::presets::set_preset_favorite,
            commands::presets::restore_builtin_presets,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::settings::reset_all_data,
            commands::settings::export_backup,
            commands::settings::import_backup,
            commands::displays::list_displays,
            commands::shortcuts::list_shortcuts,
            commands::shortcuts::set_shortcut,
            commands::shortcuts::reset_shortcuts,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Crosshair Studio");

    app.run(|app, event| {
        if let RunEvent::Exit = event {
            // Remove the crosshair from the screen before the process ends.
            if let Some(core) = app.try_state::<AppCore>() {
                core.overlay.shutdown();
            }
        }
    });
}
