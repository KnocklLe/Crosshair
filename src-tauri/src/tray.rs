//! Notification-area icon and menu.

use std::sync::Mutex;

use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

use crate::core::AppCore;
use crate::crosshair::SpecKind;

const TRAY_ID: &str = "main-tray";

struct Labels {
    show: &'static str,
    overlay: &'static str,
    dot: &'static str,
    next: &'static str,
    quit: &'static str,
}

const EN: Labels = Labels {
    show: "Show Crosshair Studio",
    overlay: "Overlay visible",
    dot: "Center Dot mode",
    next: "Next profile",
    quit: "Quit",
};

const FR: Labels = Labels {
    show: "Afficher Crosshair Studio",
    overlay: "Overlay visible",
    dot: "Mode point central",
    next: "Profil suivant",
    quit: "Quitter",
};

/// Language resolved by the interface ("en" / "fr").
pub struct UiLanguage(pub Mutex<String>);

struct TrayItems {
    show: MenuItem<Wry>,
    overlay: CheckMenuItem<Wry>,
    dot: CheckMenuItem<Wry>,
    next: MenuItem<Wry>,
    quit: MenuItem<Wry>,
}

fn labels(app: &AppHandle) -> &'static Labels {
    let lang = app
        .try_state::<UiLanguage>()
        .and_then(|l| l.0.lock().ok().map(|s| s.clone()))
        .unwrap_or_default();
    if lang == "fr" {
        &FR
    } else {
        &EN
    }
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let l = labels(app);
    let items = TrayItems {
        show: MenuItem::with_id(app, "show", l.show, true, None::<&str>)?,
        overlay: CheckMenuItem::with_id(app, "toggle_overlay", l.overlay, true, false, None::<&str>)?,
        dot: CheckMenuItem::with_id(app, "toggle_dot", l.dot, true, false, None::<&str>)?,
        next: MenuItem::with_id(app, "next_profile", l.next, true, None::<&str>)?,
        quit: MenuItem::with_id(app, "quit", l.quit, true, None::<&str>)?,
    };
    let sep1 = PredefinedMenuItem::separator(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&items.show, &sep1, &items.overlay, &items.dot, &items.next, &sep2, &items.quit])?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Crosshair Studio")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            let app = app.clone();
            let id = event.id().as_ref().to_string();
            match id.as_str() {
                "show" => show_main_window(&app),
                "quit" => app.exit(0),
                other => {
                    let action = match other {
                        "toggle_overlay" => crate::shortcuts::ShortcutAction::ToggleOverlay,
                        "toggle_dot" => crate::shortcuts::ShortcutAction::ToggleDot,
                        "next_profile" => crate::shortcuts::ShortcutAction::NextProfile,
                        _ => return,
                    };
                    tauri::async_runtime::spawn_blocking(move || crate::commands::shortcuts::run_action(&app, action, "tray"));
                }
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    app.manage(Mutex::new(items));
    refresh(app);
    Ok(())
}

/// Updates checks, labels and tooltip. Must run on the main thread.
pub fn refresh(app: &AppHandle) {
    let Some(core) = app.try_state::<AppCore>() else { return };
    let Some(items) = app.try_state::<Mutex<TrayItems>>() else { return };
    let Ok(items) = items.lock() else { return };
    let status = core.overlay.status();
    let profile = core.active_profile();
    let l = labels(app);
    let _ = items.show.set_text(l.show);
    let _ = items.overlay.set_text(l.overlay);
    let _ = items.dot.set_text(l.dot);
    let _ = items.next.set_text(l.next);
    let _ = items.quit.set_text(l.quit);
    let _ = items.overlay.set_checked(status.requested);
    let _ = items.dot.set_checked(profile.spec.kind == SpecKind::Dot);
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let state = if status.visible { "ON" } else { "OFF" };
        let _ = tray.set_tooltip(Some(format!("Crosshair Studio — {} ({state})", profile.name)));
    }
}

/// Non-blocking: queues a refresh on the main thread (safe from any thread).
pub fn schedule_refresh(app: &AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || refresh(&handle));
}
