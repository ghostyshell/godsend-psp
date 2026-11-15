//! Error type shared across the engine.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("zip: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("json: {0}")]
    Json(#[from] serde_json::Error),

    #[error("http: {0}")]
    Http(String),

    #[error(transparent)]
    Ureq(#[from] ureq::Error),

    #[error("no mounted volume with a PSP folder was found; connect the PSP and enter USB mode")]
    NoDevice,

    #[error("device required: {0}")]
    DeviceRequired(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("invalid data: {0}")]
    Invalid(String),

    #[error("job {0} was cancelled")]
    Cancelled(u64),

    #[error("ffmpeg: {0}")]
    Ffmpeg(String),

    #[error("{0}")]
    Other(String),
}

impl Error {
    pub fn other(msg: impl Into<String>) -> Self {
        Error::Other(msg.into())
    }
}

/// Tauri commands require `E: Serialize`; serialize the error as its message.
impl serde::Serialize for Error {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl std::fmt::Display for crate::transfer::JobState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            crate::transfer::JobState::Running => "running",
            crate::transfer::JobState::Done => "done",
            crate::transfer::JobState::Failed => "failed",
            crate::transfer::JobState::Cancelled => "cancelled",
        };
        f.write_str(text)
    }
}
