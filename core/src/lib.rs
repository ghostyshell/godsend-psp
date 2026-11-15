//! Framework-agnostic engine for managing a PSP over USB mass storage.
//!
//! Everything in `godsend-core` works on plain filesystem paths so it can back
//! a CLI, a Tauri desktop app, or a headless service.

pub mod config;
pub mod device;
pub mod doctor;
pub mod error;
pub mod homebrew;
pub mod iso;
pub mod cso;
pub mod library;
pub mod media;
pub mod pbp;
pub mod plugins;
pub mod sfo;
pub mod savedata;
pub mod themes;
pub mod transfer;

pub use config::Config;
pub use device::{PspPaths, Volume};
pub use error::{Error, Result};
pub use transfer::{Job, JobManager, JobState};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Product name used for data directories and window titles.
pub const PRODUCT: &str = "GODsend PSP";
