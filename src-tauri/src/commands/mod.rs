//! Tauri commands: thin wrappers that validate inputs, call the services in
//! [`crate::core`] and notify the interface. Commands that talk to the overlay
//! thread, the database or a dialog are `async` so they never block the main
//! (UI) thread.

pub mod app;
pub mod crosshair;
pub mod displays;
pub mod files;
pub mod overlay;
pub mod presets;
pub mod profiles;
pub mod settings;
pub mod shortcuts;
