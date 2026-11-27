//! Overlay: a native Win32 window, independent from the main interface,
//! that displays the active crosshair at the centre of the selected monitor.

pub mod controller;
#[cfg(windows)]
pub mod engine;

pub use controller::{OverlayEvent, OverlayOptions, OverlayStatus};
#[cfg(windows)]
pub use engine::{EventSink, OverlayEngine};
