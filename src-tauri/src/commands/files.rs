//! Native file dialogs and file access, done in Rust so the WebView never
//! needs filesystem permissions.

use std::path::PathBuf;

use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

use crate::error::{AppError, AppResult};
use crate::profiles::transfer::{parse_import, ImportBundle, MAX_IMPORT_BYTES};

fn dialog(app: &AppHandle, title: &str) -> tauri_plugin_dialog::FileDialogBuilder<tauri::Wry> {
    let mut d = app.dialog().file().set_title(title).add_filter("Crosshair Studio (JSON)", &["json"]);
    if let Some(w) = app.get_webview_window("main") {
        d = d.set_parent(&w);
    }
    d
}

/// Asks where to save, then writes `contents`. `None` when cancelled.
pub fn save_json(app: &AppHandle, title: &str, file_name: &str, contents: &str) -> AppResult<Option<String>> {
    let Some(path) = dialog(app, title).set_file_name(file_name).blocking_save_file() else {
        return Ok(None);
    };
    let mut path: PathBuf = path
        .as_path()
        .ok_or_else(|| AppError::System("Unsupported destination".into()))?
        .to_path_buf();
    if path.extension().is_none() {
        path.set_extension("json");
    }
    std::fs::write(&path, contents)?;
    Ok(Some(path.display().to_string()))
}

/// Asks for a file, then reads and validates it. `None` when cancelled.
pub fn pick_import(app: &AppHandle, title: &str) -> AppResult<Option<ImportBundle>> {
    let Some(path) = dialog(app, title).blocking_pick_file() else {
        return Ok(None);
    };
    let path = path
        .as_path()
        .ok_or_else(|| AppError::System("Unsupported file location".into()))?
        .to_path_buf();
    let len = std::fs::metadata(&path)?.len() as usize;
    if len > MAX_IMPORT_BYTES {
        return Err(AppError::Import(format!(
            "The file is too large ({} KB, maximum {} KB).",
            len / 1024,
            MAX_IMPORT_BYTES / 1024
        )));
    }
    let bytes = std::fs::read(&path)?;
    parse_import(&bytes).map(Some)
}

/// `crosshair-studio-profiles-2026-11-27.json`
pub fn dated_file_name(prefix: &str) -> String {
    let days = crate::util::now_ms() / 86_400_000;
    let (y, m, d) = civil_from_days(days);
    format!("{prefix}-{y:04}-{m:02}-{d:02}.json")
}

/// Days since 1970-01-01 → (year, month, day). Howard Hinnant's algorithm.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(20_419), (2025, 11, 27));
        assert_eq!(civil_from_days(11_016), (2000, 2, 29));
    }
}
