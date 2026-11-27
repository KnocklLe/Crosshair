use crate::displays::{self, DisplayInfo};

#[tauri::command]
pub async fn list_displays() -> Vec<DisplayInfo> {
    displays::list()
}
