//! godsend-psp: headless CLI over the same engine as the desktop app.

use clap::{Parser, Subcommand, ValueEnum};
use godsend_core::{
    config::Config,
    device,
    homebrew::{self, CatalogEntry},
    library,
    media::{self, AudioFormat},
    plugins::{self, Section},
    savedata, themes,
    transfer::JobManager,
};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

#[derive(Parser)]
#[command(
    name = "godsend-psp",
    version,
    about = "Manage a PSP from the terminal: games, homebrew, music, videos, saves, mods"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Copy, ValueEnum)]
enum SectionArg {
    Game,
    Vsh,
    Pops,
}

impl From<SectionArg> for Section {
    fn from(value: SectionArg) -> Self {
        match value {
            SectionArg::Game => Section::Game,
            SectionArg::Vsh => Section::Vsh,
            SectionArg::Pops => Section::Pops,
        }
    }
}

#[derive(Subcommand)]
enum Command {
    /// List mounted volumes and flag the PSP ones.
    Devices,
    /// Print the detected memory stick layout.
    Paths,
    /// List installed games and disc images.
    Scan,
    /// Print an installed game's PARAM.SFO metadata.
    Game { id_or_folder: String },
    /// Import and install a homebrew zip onto the stick.
    Install {
        zip: PathBuf,
        /// Force a layout: auto | ark | psp_root | eboot
        #[arg(long, default_value = "auto")]
        layout: String,
    },
    /// Show the bundled homebrew catalog (network refresh with --refresh).
    Catalog {
        #[arg(long)]
        refresh: bool,
    },
    /// Install a catalog entry by id.
    CatalogInstall { id: String },
    /// Push music into PSP/MUSIC (converts with ffmpeg unless --raw).
    Music {
        files: Vec<PathBuf>,
        #[arg(long)]
        raw: bool,
        #[arg(long, default_value = "mp3")]
        format: String,
    },
    /// Push videos into PSP/VIDEO (converts to PSP AVC unless --raw).
    Video {
        files: Vec<PathBuf>,
        #[arg(long)]
        raw: bool,
        /// Also write a 160x120 .THM thumbnail next to each video.
        #[arg(long, default_value_t = true)]
        thm: bool,
    },
    /// Import an ISO/CSO/PBP into the ISO/ folder.
    IsoImport {
        file: PathBuf,
        /// Compress to CSO on the way in.
        #[arg(long)]
        cso: bool,
    },
    /// Convert ISO to CSO (or CSO to ISO).
    Convert {
        file: PathBuf,
        #[arg(long, default_value = "6")]
        level: u32,
    },
    /// List save folders, or backup/restore saves.
    Saves {
        #[command(subcommand)]
        action: Option<SavesAction>,
    },
    /// List or toggle CFW plugins (seplugins).
    Plugins {
        #[command(subcommand)]
        action: Option<PluginsAction>,
    },
    /// List or install XMB themes (.ptf/.ctf).
    Themes {
        #[command(subcommand)]
        action: Option<ThemesAction>,
    },
    /// Environment check (ffmpeg, data dir, device).
    Doctor,
    /// Serve the loopback MCP endpoint (127.0.0.1) until Ctrl-C.
    Mcp,
}

#[derive(Subcommand)]
enum SavesAction {
    /// Zip saves to a file (default: all saves).
    Backup {
        #[arg(long)]
        out: PathBuf,
        /// Specific folder names; empty = all.
        folders: Vec<String>,
    },
    /// Restore a backup zip onto the stick.
    Restore { zip: PathBuf },
    /// Delete a save folder.
    Delete { folder: String },
}

#[derive(Subcommand)]
enum PluginsAction {
    /// List plugin entries across game/vsh/pops.
    List,
    /// Enable or disable one plugin in a section.
    Toggle {
        section: SectionArg,
        plugin_path: String,
        #[arg(long)]
        disable: bool,
    },
    /// Copy a .prx into seplugins and enable it.
    Add { section: SectionArg, prx: PathBuf },
    /// Remove a plugin line from a section.
    Remove { section: SectionArg, plugin_path: String },
}

#[derive(Subcommand)]
enum ThemesAction {
    List,
    Install { file: PathBuf },
    Delete { name: String },
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "godsend_core=warn".into()),
        )
        .init();
    let cli = Cli::parse();
    if let Err(err) = run(cli) {
        eprintln!("error: {}", err);
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> godsend_core::Result<()> {
    match cli.command {
        Command::Devices => {
            let volumes = device::scan_volumes();
            if volumes.is_empty() {
                println!("no mounted volumes found");
            }
            for v in volumes {
                let marker = if v.is_psp { "PSP" } else { "   " };
                println!(
                    "{} {:<28} {:>8} free / {:>10} total",
                    marker,
                    v.mount,
                    human(v.free_bytes),
                    human(v.total_bytes)
                );
            }
            Ok(())
        }
        Command::Paths => {
            let (_, paths) = device::require_psp()?;
            for (name, path) in [
                ("root", paths.root.clone()),
                ("game", paths.game()),
                ("savedata", paths.savedata()),
                ("music", paths.music()),
                ("video", paths.video()),
                ("theme", paths.theme()),
                ("iso", paths.iso()),
                ("seplugins", paths.seplugins()),
            ] {
                println!("{:<10} {}", name, path.display());
            }
            Ok(())
        }
        Command::Scan => {
            let (_, paths) = device::require_psp()?;
            let lib = library::scan(&paths)?;
            if !lib.games.is_empty() {
                println!("PSP/GAME:");
                for g in &lib.games {
                    let kind = match g.kind {
                        library::GameKind::Digital => "digital",
                        library::GameKind::Homebrew => "homebrew",
                        library::GameKind::Loader => "loader",
                    };
                    println!(
                        "  {:<24} {:<40} {:<9} {:>9} {}",
                        g.folder,
                        truncate(&g.title, 40),
                        kind,
                        human(g.size_bytes),
                        g.title_id.as_deref().unwrap_or("-")
                    );
                }
            }
            if !lib.images.is_empty() {
                println!("ISO/:");
                for i in &lib.images {
                    println!(
                        "  {:<36} {:<4} {:>10} {}",
                        i.file,
                        i.format,
                        human(i.size_bytes),
                        i.title.as_deref().unwrap_or("-")
                    );
                }
            }
            if lib.games.is_empty() && lib.images.is_empty() {
                println!("nothing installed");
            }
            Ok(())
        }
        Command::Game { id_or_folder } => {
            let (_, paths) = device::require_psp()?;
            let games = library::scan_games(&paths)?;
            let game = games
                .iter()
                .find(|g| g.folder == id_or_folder || g.id == id_or_folder)
                .ok_or_else(|| godsend_core::Error::NotFound(id_or_folder))?;
            println!("{}", serde_json::to_string_pretty(game)?);
            Ok(())
        }
        Command::Install { zip, layout } => {
            let (_, paths) = device::require_psp()?;
            let parsed = match layout.as_str() {
                "ark" => homebrew::Layout::Ark,
                "psp_root" => homebrew::Layout::PspRoot,
                "eboot" => homebrew::Layout::Eboot,
                _ => homebrew::Layout::Auto,
            };
            let report = homebrew::install_zip(&zip, &paths, parsed, None)?;
            println!("layout: {}", report.layout_used);
            for path in &report.installed {
                println!("  installed {}", path);
            }
            for path in &report.skipped {
                println!("  skipped   {}", path);
            }
            Ok(())
        }
        Command::Catalog { refresh } => {
            let mut catalog = homebrew::load_catalog();
            if refresh {
                let updated = homebrew::refresh_catalog(&mut catalog)?;
                println!("refreshed {} entries", updated);
            }
            for entry in &catalog {
                let release = entry.latest_tag.as_deref().unwrap_or("? ");
                let zip = if entry.zip_url.is_some() { "zip" } else { "no zip" };
                println!(
                    "{:<12} {:<16} {:<10} {:<8} {}",
                    entry.id, entry.name, entry.category, release, zip
                );
            }
            Ok(())
        }
        Command::CatalogInstall { id } => {
            let (_, paths) = device::require_psp()?;
            let mgr = JobManager::new();
            let mut catalog = homebrew::load_catalog();
            homebrew::refresh_catalog(&mut catalog).ok();
            let entry: CatalogEntry = catalog
                .iter()
                .find(|e| e.id == id)
                .cloned()
                .ok_or_else(|| godsend_core::Error::NotFound(id))?;
            let entry2 = entry.clone();
            let paths2 = paths.clone();
            let job_id = mgr.spawn("install", &entry.name, |_| {}, move |ctx| {
                homebrew::install_with_job(&entry2, &paths2, ctx).map(|_| ())
            });
            loop {                let job = mgr
                    .get(job_id)
                    .ok_or_else(|| godsend_core::Error::NotFound("job".into()))?;
                if job.state != godsend_core::transfer::JobState::Running {
                    match job.state {
                        godsend_core::transfer::JobState::Done => println!("installed {}", job.label),
                        other => return Err(godsend_core::Error::Other(format!("{}: {}", other, job.message))),
                    }
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(400));
            }
            Ok(())
        }
        Command::Music { files, raw, format } => {
            let (config, paths) = env()?;
            let fmt = match format.as_str() {
                "m4a" => AudioFormat::M4a,
                _ => AudioFormat::Mp3,
            };
            push_music(&config, &paths, &files, raw, fmt)
        }
        Command::Video { files, raw, thm } => {
            let (config, paths) = env()?;
            push_video(&config, &paths, &files, raw, thm)
        }
        Command::IsoImport { file, cso } => {
            let (_, paths) = device::require_psp()?;
            std::fs::create_dir_all(paths.iso())?;
            let dest_name = file
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .ok_or_else(|| godsend_core::Error::Invalid("bad file".into()))?;
            let dest = if cso {
                paths.iso().join(dest_name.replace(".iso", ".cso"))
            } else {
                paths.iso().join(dest_name)
            };
            if cso {
                let cancel = AtomicBool::new(false);
                godsend_core::cso::iso_to_cso(&file, &dest, config_cso_level(), &mut |done, total| {
                    print_progress(done, total);
                }, &cancel)?;
                println!();
            } else {
                std::fs::copy(&file, &dest)?;
            }
            println!("imported {}", dest.display());
            Ok(())
        }
        Command::Convert { file, level } => {
            let out = if file.extension().map(|e| e.to_string_lossy().to_lowercase()) == Some("cso".into()) {
                file.with_extension("iso")
            } else {
                file.with_extension("cso")
            };
            let cancel = AtomicBool::new(false);
            if out.extension().map(|e| e.to_string_lossy().to_lowercase()) == Some("cso".into()) {
                godsend_core::cso::iso_to_cso(&file, &out, level, &mut |done, total| print_progress(done, total), &cancel)?;
            } else {
                godsend_core::cso::cso_to_iso(&file, &out, &mut |done, total| print_progress(done, total), &cancel)?;
            }
            println!();
            println!("wrote {}", out.display());
            Ok(())
        }
        Command::Saves { action } => match action {
            None => {
                let (_, paths) = device::require_psp()?;
                for save in savedata::list(&paths)? {
                    println!(
                        "{:<28} {:<34} {:>9} {}",
                        save.folder,
                        truncate(
                            save.title.as_deref().or(save.subtitle.as_deref()).unwrap_or("-"),
                            34
                        ),
                        human(save.size_bytes),
                        save.title_id.as_deref().unwrap_or("-")
                    );
                }
                Ok(())
            }
            Some(SavesAction::Backup { out, folders }) => {
                let (_, paths) = device::require_psp()?;
                let written = savedata::backup(&paths, &folders, &out, None)?;
                println!("wrote {} ({} bytes) to {}", out.display(), written, out.display());
                Ok(())
            }
            Some(SavesAction::Restore { zip }) => {
                let (_, paths) = device::require_psp()?;
                let files = savedata::restore(&paths, &zip, None)?;
                println!("restored {} files", files);
                Ok(())
            }
            Some(SavesAction::Delete { folder }) => {
                let (_, paths) = device::require_psp()?;
                savedata::delete_save(&paths, &folder)?;
                println!("deleted {}", folder);
                Ok(())
            }
        },
        Command::Plugins { action } => match action {
            None | Some(PluginsAction::List) => {
                let (_, paths) = device::require_psp()?;
                for section in [Section::Game, Section::Vsh, Section::Pops] {
                    let entries = plugins::list(&paths, section)?;
                    if entries.is_empty() {
                        continue;
                    }
                    println!("{}:", section.file_name());
                    for p in entries {
                        let state = if p.enabled { "on " } else { "off" };
                        let exists = if p.exists { "" } else { " (missing)" };
                        println!("  [{}] {}{}", state, p.path, exists);
                    }
                }
                Ok(())
            }
            Some(PluginsAction::Toggle { section, plugin_path, disable }) => {
                let (_, paths) = device::require_psp()?;
                plugins::toggle(&paths, section.into(), &plugin_path, !disable)?;
                println!("{} {}", if disable { "disabled" } else { "enabled" }, plugin_path);
                Ok(())
            }
            Some(PluginsAction::Add { section, prx }) => {
                let (_, paths) = device::require_psp()?;
                let entry = plugins::add(&paths, section.into(), &prx)?;
                println!("installed {}", entry.path);
                Ok(())
            }
            Some(PluginsAction::Remove { section, plugin_path }) => {
                let (_, paths) = device::require_psp()?;
                plugins::remove(&paths, section.into(), &plugin_path)?;
                println!("removed {}", plugin_path);
                Ok(())
            }
        },
        Command::Themes { action } => match action {
            None | Some(ThemesAction::List) => {
                let (_, paths) = device::require_psp()?;
                for theme in themes::list(&paths)? {
                    println!("{:<32} {:<4} {:>9}", theme.name, theme.format, human(theme.size_bytes));
                }
                Ok(())
            }
            Some(ThemesAction::Install { file }) => {
                let (_, paths) = device::require_psp()?;
                let entry = themes::install(&paths, &file)?;
                println!("installed {}", entry.name);
                Ok(())
            }
            Some(ThemesAction::Delete { name }) => {
                let (_, paths) = device::require_psp()?;
                themes::delete(&paths, &name)?;
                println!("deleted {}", name);
                Ok(())
            }
        },
        Command::Doctor => {
            let config = Config::load();
            for check in godsend_core::doctor::run(&config) {
                println!("[{}] {:<14} {}", if check.ok { "ok " } else { "!! " }, check.name, check.detail);
            }
            Ok(())
        }
        Command::Mcp => {
            let config = Config::load();
            let endpoint = godsend_core::mcp::ServerConfig::from_config(&config).url();
            let _handle = godsend_core::mcp::serve(&config)?;
            println!("MCP server listening on {}", endpoint);
            println!("POST JSON-RPC 2.0 (MCP {}); press Ctrl-C to stop.", godsend_core::mcp::PROTOCOL_VERSION);
            loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }
    }
}

fn env() -> godsend_core::Result<(Config, device::PspPaths)> {
    let config = Config::load();
    let (_, paths) = device::require_psp()?;
    Ok((config, paths))
}

fn config_cso_level() -> u32 {
    Config::load().cso_level
}

fn push_music(
    config: &Config,
    paths: &device::PspPaths,
    files: &[PathBuf],
    raw: bool,
    format: AudioFormat,
) -> godsend_core::Result<()> {
    let music = paths.music();
    std::fs::create_dir_all(&music)?;
    let ffmpeg = media::find_ffmpeg(&config.ffmpeg_path);
    let cancel = AtomicBool::new(false);
    for file in files {
        if !file.is_file() {
            return Err(godsend_core::Error::NotFound(file.display().to_string()));
        }
        let name = media::fat_safe_name(
            file.file_stem().map(|n| n.to_string_lossy().to_string()).as_deref().unwrap_or("track"),
        );
        if raw || media::is_psp_audio(file) || ffmpeg.is_none() {
            let ext = file.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_else(|| "mp3".into());
            let dest = music.join(format!("{}.{}", name, ext));
            std::fs::copy(file, &dest)?;
            println!("copied {}", dest.display());
        } else {
            let ffmpeg = ffmpeg.as_ref().expect("checked above");
            let dest = music.join(format!("{}.{}", name, format.ext()));
            let probe = media::find_ffprobe(&config.ffprobe_path);
            media::convert_audio(ffmpeg, probe.as_deref(), file, &dest, &format, &cancel, &mut |done, total| {
                print!("{}: {}", file.display(), percent(done, total));
                let _ = std::io::Write::flush(&mut std::io::stdout());
            })?;
            println!("\ncopied {}", dest.display());
        }
    }
    Ok(())
}

fn push_video(
    config: &Config,
    paths: &device::PspPaths,
    files: &[PathBuf],
    raw: bool,
    thm: bool,
) -> godsend_core::Result<()> {
    let video = paths.video();
    std::fs::create_dir_all(&video)?;
    let ffmpeg = media::find_ffmpeg(&config.ffmpeg_path);
    let cancel = AtomicBool::new(false);
    for file in files {
        if !file.is_file() {
            return Err(godsend_core::Error::NotFound(file.display().to_string()));
        }
        let name = media::fat_safe_name(
            file.file_stem().map(|n| n.to_string_lossy().to_string()).as_deref().unwrap_or("video"),
        );
        if raw || media::is_psp_video(file) || ffmpeg.is_none() {
            let dest = video.join(format!("{}.mp4", name));
            std::fs::copy(file, &dest)?;
            println!("copied {}", dest.display());
        } else {
            let ffmpeg = ffmpeg.as_ref().expect("checked above");
            let dest = video.join(format!("{}.mp4", name));
            let thm_dest = if thm { Some(video.join(format!("{}.THM", name))) } else { None };
            let probe = media::find_ffprobe(&config.ffprobe_path);
            let quality = media::VideoQuality {
                preset: config.video_preset.clone(),
        crf: config.video_crf,
                fps: config.video_fps,
                audio_kbps: config.video_audio_kbps,
                resolution: config.video_resolution.clone(),
            };
            media::convert_video(ffmpeg, probe.as_deref(), file, &dest, thm_dest.as_deref(), &quality, &cancel, &mut |done, total| {
                print!("{}: {}", file.display(), percent(done, total));
                let _ = std::io::Write::flush(&mut std::io::stdout());
            })?;
            println!("\ncopied {}", dest.display());
        }
    }
    Ok(())
}

fn print_progress(done: u64, total: u64) {
    print!("\r{}", percent(done, total));
    let _ = std::io::Write::flush(&mut std::io::stdout());
}

fn percent(done: u64, total: u64) -> String {
    if total == 0 {
        return "...".into();
    }
    format!("{}%", done * 100 / total)
}

fn human(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[0])
    } else {
        format!("{:.1} {}", value, UNITS[unit])
    }
}

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_string()
    } else {
        format!("{}...", text.chars().take(max - 1).collect::<String>())
    }
}

// Keep Path import used on all platforms.
#[allow(unused)]
fn _keep_path(_: &Path) {}
