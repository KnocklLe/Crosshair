//! Global keyboard shortcuts: model, parsing and validation.
//!
//! Registration with the OS (RegisterHotKey, via the official Tauri plugin)
//! lives in [`manager`]. This file is pure and unit-tested.

pub mod manager;

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShortcutAction {
    ToggleOverlay,
    ToggleDot,
    NextProfile,
}

impl ShortcutAction {
    pub const ALL: [ShortcutAction; 3] = [
        ShortcutAction::ToggleOverlay,
        ShortcutAction::ToggleDot,
        ShortcutAction::NextProfile,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            ShortcutAction::ToggleOverlay => "toggle_overlay",
            ShortcutAction::ToggleDot => "toggle_dot",
            ShortcutAction::NextProfile => "next_profile",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|a| a.as_str() == s)
    }

    pub fn default_accelerator(&self) -> &'static str {
        match self {
            ShortcutAction::ToggleOverlay => "Ctrl+Shift+H",
            ShortcutAction::ToggleDot => "Ctrl+Shift+D",
            ShortcutAction::NextProfile => "Ctrl+Shift+P",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutBinding {
    pub action: ShortcutAction,
    pub accelerator: String,
    pub enabled: bool,
}

impl ShortcutBinding {
    pub fn default_for(action: ShortcutAction) -> Self {
        Self { action, accelerator: action.default_accelerator().into(), enabled: true }
    }
}

pub fn default_bindings() -> Vec<ShortcutBinding> {
    ShortcutAction::ALL.into_iter().map(ShortcutBinding::default_for).collect()
}

/// Binding plus the real registration result reported by the OS.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutStatus {
    pub action: ShortcutAction,
    pub accelerator: String,
    pub enabled: bool,
    pub registered: bool,
    pub error: Option<String>,
}

const MODIFIERS: [(&str, &[&str]); 4] = [
    ("Ctrl", &["ctrl", "control"]),
    ("Alt", &["alt", "option"]),
    ("Shift", &["shift"]),
    ("Super", &["super", "win", "windows", "meta", "cmd"]),
];

const NAMED_KEYS: &[&str] = &[
    "Space", "Enter", "Tab", "Backspace", "Delete", "Insert", "Home", "End", "PageUp", "PageDown",
    "Up", "Down", "Left", "Right", "Minus", "Equal", "Comma", "Period", "Slash", "Backslash",
    "Semicolon", "Quote", "BracketLeft", "BracketRight", "Backquote", "Pause", "PrintScreen",
    "ScrollLock", "NumpadAdd", "NumpadSubtract", "NumpadMultiply", "NumpadDivide",
    "NumpadDecimal", "NumpadEnter",
];

fn normalize_key(token: &str) -> Option<String> {
    let t = token.trim();
    if t.len() == 1 {
        let c = t.chars().next()?;
        if c.is_ascii_alphanumeric() {
            return Some(c.to_ascii_uppercase().to_string());
        }
        return None;
    }
    let lower = t.to_ascii_lowercase();
    if let Some(n) = lower.strip_prefix('f') {
        if let Ok(n) = n.parse::<u8>() {
            if (1..=24).contains(&n) {
                return Some(format!("F{n}"));
            }
        }
    }
    if let Some(n) = lower.strip_prefix("numpad").or_else(|| lower.strip_prefix("num")) {
        if let Ok(n) = n.parse::<u8>() {
            if n <= 9 {
                return Some(format!("Numpad{n}"));
            }
        }
    }
    let alias = match lower.as_str() {
        "arrowup" => "up",
        "arrowdown" => "down",
        "arrowleft" => "left",
        "arrowright" => "right",
        "return" => "enter",
        "del" => "delete",
        "ins" => "insert",
        other => other,
    };
    NAMED_KEYS
        .iter()
        .find(|k| k.eq_ignore_ascii_case(alias))
        .map(|k| k.to_string())
}

/// Validates and canonicalizes an accelerator: `" ctrl + shift + h "` →
/// `"Ctrl+Shift+H"`. At least one modifier and exactly one key are required.
pub fn normalize_accelerator(input: &str) -> AppResult<String> {
    let tokens: Vec<&str> = input.split('+').map(str::trim).filter(|t| !t.is_empty()).collect();
    if tokens.is_empty() {
        return Err(AppError::Shortcut("The shortcut is empty.".into()));
    }
    let mut mods = [false; 4];
    let mut key: Option<String> = None;
    for tok in tokens {
        let lower = tok.to_ascii_lowercase();
        if let Some(i) = MODIFIERS.iter().position(|(_, names)| names.contains(&lower.as_str())) {
            if mods[i] {
                return Err(AppError::Shortcut(format!("Modifier \"{tok}\" is repeated.")));
            }
            mods[i] = true;
            continue;
        }
        let k = normalize_key(tok).ok_or_else(|| AppError::Shortcut(format!("Unsupported key \"{tok}\".")))?;
        if key.replace(k).is_some() {
            return Err(AppError::Shortcut("A shortcut can only contain one main key.".into()));
        }
    }
    let key = key.ok_or_else(|| AppError::Shortcut("Add a main key after the modifiers.".into()))?;
    if !mods.iter().any(|m| *m) {
        return Err(AppError::Shortcut(
            "A global shortcut needs at least one modifier (Ctrl, Alt, Shift or Win).".into(),
        ));
    }
    if mods == [false, false, true, false] && key.len() == 1 {
        return Err(AppError::Shortcut("Shift + a letter or digit would block normal typing.".into()));
    }
    let mut parts: Vec<&str> = MODIFIERS.iter().zip(mods).filter(|(_, on)| *on).map(|((n, _), _)| *n).collect();
    parts.push(&key);
    Ok(parts.join("+"))
}

/// Returns the action already using `accelerator`, if any (enabled bindings only).
pub fn find_conflict(bindings: &[ShortcutBinding], action: ShortcutAction, accelerator: &str) -> Option<ShortcutAction> {
    bindings
        .iter()
        .find(|b| b.action != action && b.enabled && b.accelerator.eq_ignore_ascii_case(accelerator))
        .map(|b| b.action)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        let d = default_bindings();
        assert_eq!(d.len(), 3);
        assert_eq!(d[0].accelerator, "Ctrl+Shift+H");
        for b in &d {
            assert_eq!(normalize_accelerator(&b.accelerator).unwrap(), b.accelerator);
        }
    }

    #[test]
    fn normalization() {
        assert_eq!(normalize_accelerator(" ctrl + shift + h ").unwrap(), "Ctrl+Shift+H");
        assert_eq!(normalize_accelerator("Shift+Control+F5").unwrap(), "Ctrl+Shift+F5");
        assert_eq!(normalize_accelerator("win+alt+ArrowUp").unwrap(), "Alt+Super+Up");
        assert_eq!(normalize_accelerator("Ctrl+num5").unwrap(), "Ctrl+Numpad5");
        assert_eq!(normalize_accelerator("Alt+f24").unwrap(), "Alt+F24");
        assert_eq!(normalize_accelerator("Shift+F2").unwrap(), "Shift+F2");
    }

    #[test]
    fn invalid_accelerators() {
        for bad in ["", "H", "Ctrl+Shift", "Ctrl+H+J", "Ctrl+Ctrl+H", "Ctrl+F25", "Ctrl+é", "Shift+A", "Ctrl+Banana"] {
            assert!(normalize_accelerator(bad).is_err(), "{bad} should be rejected");
        }
    }

    #[test]
    fn conflicts_between_actions() {
        let mut b = default_bindings();
        assert_eq!(find_conflict(&b, ShortcutAction::ToggleDot, "ctrl+shift+h"), Some(ShortcutAction::ToggleOverlay));
        assert_eq!(find_conflict(&b, ShortcutAction::ToggleOverlay, "Ctrl+Shift+H"), None);
        b[0].enabled = false;
        assert_eq!(find_conflict(&b, ShortcutAction::ToggleDot, "Ctrl+Shift+H"), None);
    }

    #[test]
    fn action_names_round_trip() {
        for a in ShortcutAction::ALL {
            assert_eq!(ShortcutAction::parse(a.as_str()), Some(a));
        }
    }
}
