use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cryptographic operation failed: {0}")]
    Crypto(String),

    #[error("invalid key material: {0}")]
    InvalidKey(String),

    #[error("serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("identity error: {0}")]
    Identity(String),

    #[error("storage error: {0}")]
    Storage(String),

    #[error("network error: {0}")]
    Network(String),

    #[error("message error: {0}")]
    Message(String),

    #[error("governance error: {0}")]
    Governance(String),

    #[error("permission denied: {0}")]
    PermissionDenied(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("expired: {0}")]
    Expired(String),

    #[error("{0}")]
    Other(String),
}
