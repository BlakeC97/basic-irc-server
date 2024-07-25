use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Serialize, Deserialize, Debug, Error)]
pub enum AuthResponse {
    // We don't construct this as an error ever
    #[error("")]
    Success,
    #[error("{0}")]
    Error(String),
}