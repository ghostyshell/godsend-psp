//! Framework-agnostic engine for managing a PSP over USB mass storage.
//!
//! Everything in `godsend-core` works on plain filesystem paths so it can back
//! a CLI, a Tauri desktop app, or a headless service.

pub mod catalog;
pub mod cfw;
pub mod config;
pub mod debrid;
pub mod device;
pub mod doctor;
pub mod error;
pub mod fm;
pub mod homebrew;
pub mod iso;
pub mod cso;
pub mod library;
pub mod media;
pub mod mcp;
pub mod mods;
pub mod aria2;
pub mod pbp;
pub mod plugins;
pub mod sfo;
pub mod sources;
pub mod store;
pub mod savedata;
pub mod theme_pack;
pub mod themes;
pub mod torrent;
pub mod transfer;
pub mod update;

pub use config::Config;
pub use device::{PspPaths, Volume};
pub use error::{Error, Result};
pub use transfer::{Job, JobManager, JobState};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Product name used for data directories and window titles.
pub const PRODUCT: &str = "GODsend PSP";
