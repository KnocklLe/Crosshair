//! Crosshair engine: data model, validation, geometry, rendering and presets.
//!
//! This module has no dependency on Tauri or Win32: it is pure, deterministic
//! and fully unit-tested.

pub mod geometry;
pub mod model;
pub mod presets;
pub mod render;
pub mod validate;

pub use model::*;
