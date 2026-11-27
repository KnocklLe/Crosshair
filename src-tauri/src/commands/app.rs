use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::core::{AppCore, StateSnapshot, APP_VERSION};
use crate::crosshair::validate::{Limits, LIMITS};
use crate::diagnostics::{ProcessUsage, Sampler};
use crate::displays::{self, DisplayInfo};
use crate::error::{AppError, AppResult};
use crate::events::emit_state;
use crate::library::RecentEntry;
use crate::overlay::OverlayStatus;
use crate::profiles::Profile;
use crate::settings::Settings;
use crate::shortcuts::ShortcutStatus;
use crate::tray::{self, UiLanguage};

use super::shortcuts::ShortcutRegistry;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: &'static str,
    pub version: &'static str,
    pub tauri_version: &'static str,
    pub data_dir: String,
    pub db_path: String,
    pub schema_version: i64,
    pub launched_at_startup: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bootstrap {
    pub app: AppInfo,
    pub settings: Settings,
    pub limits: Limits,
    pub state: StateSnapshot,
    pub shortcuts: Vec<ShortcutStatus>,
    pub displays: Vec<DisplayInfo>,
}

fn app_info(core: &AppCore) -> AppResult<AppInfo> {
    Ok(AppInfo {
        name: "Crosshair Studio",
        version: APP_VERSION,
        tauri_version: tauri::VERSION,
        data_dir: core.data_dir().display().to_string(),
        db_path: core.db_path().display().to_string(),
        schema_version: core.with_storage(|s| s.schema_version())?,
        launched_at_startup: core.launched_at_startup,
    })
}

/// Everything the interface needs to start, in one round trip.
#[tauri::command]
pub async fn get_bootstrap(core: State<'_, AppCore>, registry: State<'_, ShortcutRegistry>) -> AppResult<Bootstrap> {
    Ok(Bootstrap {
        app: app_info(&core)?,
        settings: core.settings(),
        limits: LIMITS,
        state: core.snapshot("startup")?,
        shortcuts: registry.list(),
        displays: displays::list(),
    })
}

/// Called once the interface is rendered: shows the window (unless Windows
/// started the app and the user wants it to stay in the tray).
#[tauri::command]
pub async fn app_ready(app: AppHandle, core: State<'_, AppCore>, language: String) -> AppResult<()> {
    set_language(&app, &language);
    let settings = core.settings();
    if !(core.launched_at_startup && settings.start_minimized) {
        tray::show_main_window(&app);
    }
    Ok(())
}

fn set_language(app: &AppHandle, language: &str) {
    let lang = if language == "fr" { "fr" } else { "en" };
    if let Some(state) = app.try_state::<UiLanguage>() {
        if let Ok(mut l) = state.0.lock() {
            *l = lang.to_string();
        }
    }
    tray::schedule_refresh(app);
}

#[tauri::command]
pub async fn set_ui_language(app: AppHandle, language: String) -> AppResult<()> {
    set_language(&app, &language);
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Dashboard {
    pub state: StateSnapshot,
    pub preset_count: usize,
    pub custom_preset_count: usize,
    pub favorite_count: usize,
    pub recents: Vec<RecentEntry>,
    pub shortcuts: Vec<ShortcutStatus>,
    pub display_count: usize,
}

#[tauri::command]
pub async fn get_dashboard(core: State<'_, AppCore>, registry: State<'_, ShortcutRegistry>) -> AppResult<Dashboard> {
    let presets = core.list_presets()?;
    Ok(Dashboard {
        state: core.snapshot("ui")?,
        preset_count: presets.len(),
        custom_preset_count: presets.iter().filter(|p| !p.builtin).count(),
        favorite_count: presets.iter().filter(|p| p.favorite).count(),
        recents: core.recents(8)?,
        shortcuts: registry.list(),
        display_count: displays::list().len(),
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceInfo {
    pub process: ProcessUsage,
    pub overlay: OverlayStatus,
}

#[tauri::command]
pub async fn get_performance(core: State<'_, AppCore>, sampler: State<'_, Sampler>) -> AppResult<PerformanceInfo> {
    Ok(PerformanceInfo { process: sampler.sample(core.started_at), overlay: core.overlay.status() })
}

#[tauri::command]
pub async fn open_data_folder(core: State<'_, AppCore>) -> AppResult<()> {
    std::process::Command::new("explorer.exe")
        .arg(core.data_dir())
        .spawn()
        .map(|_| ())
        .map_err(|e| AppError::System(format!("Could not open the folder: {e}")))
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
pub async fn list_recents(core: State<'_, AppCore>, limit: Option<i64>) -> AppResult<Vec<RecentEntry>> {
    core.recents(limit.unwrap_or(8))
}

#[tauri::command]
pub async fn apply_recent(app: AppHandle, core: State<'_, AppCore>, id: i64) -> AppResult<Profile> {
    let p = core.apply_recent(id)?;
    emit_state(&app, "ui");
    Ok(p)
}
