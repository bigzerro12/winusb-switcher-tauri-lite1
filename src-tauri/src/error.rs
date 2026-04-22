//! Application error types.
//!
//! All Tauri commands return `Result<T, AppError>`.
//! `AppError` is serialized to JSON so the frontend can handle typed errors.

use serde::Serialize;

#[derive(Debug, Serialize, thiserror::Error)]
#[serde(tag = "kind", content = "message", rename_all = "camelCase")]
pub enum AppError {
    /// Bundled runtime is missing / invalid / not prepared.
    #[error("Runtime error: {0}")]
    Runtime(String),

    /// Native bridge (FFI) error.
    #[error("Bridge error: {0}")]
    Bridge(String),

    /// I/O error
    #[error("IO error: {0}")]
    Io(String),

    /// Generic/unexpected error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e.to_string())
    }
}

impl From<tokio::task::JoinError> for AppError {
    fn from(e: tokio::task::JoinError) -> Self {
        AppError::Internal(e.to_string())
    }
}

/// Shorthand Result type used throughout the app
pub type AppResult<T> = Result<T, AppError>;

/// Typed error for native bridge calls (wraps string errors from `jlink_ffi`).
#[derive(Debug, thiserror::Error)]
pub enum BridgeError {
    #[error("bridge call failed: {0}")]
    Failed(String),
}

impl From<BridgeError> for AppError {
    fn from(e: BridgeError) -> Self {
        AppError::Bridge(e.to_string())
    }
}