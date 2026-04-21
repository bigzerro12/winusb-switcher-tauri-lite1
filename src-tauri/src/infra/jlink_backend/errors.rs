use thiserror::Error;

#[derive(Debug, Error)]
pub enum BridgeError {
    #[error("bridge call failed: {0}")]
    Failed(String),
}

impl From<BridgeError> for crate::error::AppError {
    fn from(e: BridgeError) -> Self {
        crate::error::AppError::Bridge(e.to_string())
    }
}

