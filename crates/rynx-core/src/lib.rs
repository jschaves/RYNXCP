//! Shared primitives for RYNXCP: configuration, validation, cryptography and
//! the templates for every artifact the daemon renders.

pub mod audit;
pub mod config;
pub mod crypto;
pub mod paths;
pub mod policy;
pub mod programs;
pub mod templates;
pub mod validate;

pub use config::{ApiConfig, DaemonConfig};
pub use crypto::{constant_time_eq, random_hex, sha256_hex};
pub use policy::PasswordPolicy;
pub use programs::service_unit_for_program;
pub use validate::*;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("configuration error: {0}")]
    Config(String),
    #[error("filesystem error: {0}")]
    Io(String),
    #[error("cryptographic error: {0}")]
    Crypto(String),
    #[error("audit log error: {0}")]
    Audit(String),
}

impl From<std::io::Error> for CoreError {
    fn from(value: std::io::Error) -> Self {
        CoreError::Io(value.to_string())
    }
}

pub type CoreResult<T> = Result<T, CoreError>;
