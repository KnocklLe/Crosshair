//! Application-wide error type.
//!
//! Every Tauri command returns `AppResult<T>`. Errors are serialized to the
//! frontend as `{ code, message }` so the UI can show a precise message.

use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Validation(String),
    #[error("{0}")]
    NotFound(String),
    #[error("database error: {0}")]
    Storage(#[from] rusqlite::Error),
    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("file error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Overlay(String),
    #[error("{0}")]
    Shortcut(String),
    #[error("{0}")]
    Import(String),
    #[error("{0}")]
    System(String),
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            AppError::Validation(_) => "validation",
            AppError::NotFound(_) => "not_found",
            AppError::Storage(_) => "storage",
            AppError::Json(_) => "json",
            AppError::Io(_) => "io",
            AppError::Overlay(_) => "overlay",
            AppError::Shortcut(_) => "shortcut",
            AppError::Import(_) => "import",
            AppError::System(_) => "system",
        }
    }

    pub fn validation(msg: impl Into<String>) -> Self {
        AppError::Validation(msg.into())
    }

    pub fn not_found(what: impl Into<String>) -> Self {
        AppError::NotFound(what.into())
    }
}

#[derive(Serialize)]
struct ErrorPayload<'a> {
    code: &'a str,
    message: String,
}

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        ErrorPayload {
            code: self.code(),
            message: self.to_string(),
        }
        .serialize(serializer)
    }
}

impl From<tauri::Error> for AppError {
    fn from(e: tauri::Error) -> Self {
        AppError::System(e.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
