use thiserror::Error;

use revm::database_interface::DBErrorMarker;

#[derive(Error, Debug)]
pub enum GIOError {
    #[error("failed to emit gio: {0}")]
    EmitFailed(String),

    #[error("{message:?}: gio response code - {response_code:?}")]
    BadResponse { message: String, response_code: u32 },
}

impl DBErrorMarker for GIOError {}
