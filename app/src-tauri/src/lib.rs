//! Tauri command layer over `godsend-core`.
//!
//! Frontend calls these via `invoke(...)`; long jobs report progress through
//! the `job://progress` / `job://done` events, device changes through
//! `device://changed`.

use godsend_core::cfw::{self, CfwState};
use godsend_core::config::Config;
use godsend_core::device::{self, PspPaths, Volume};
use godsend_core::homebrew::{self, CatalogEntry, InstallReport};
use godsend_core::library::{self, DiscImage, GameEntry};
use godsend_core::media;
use godsend_core::plugins::{self, PluginEntry, Section};
use godsend_core::savedata::{self, SaveEntry};
use godsend_core::themes::{self, ThemeEntry};
use godsend_core::theme_pack::{self, ThemePack};
use godsend_core::transfer::{set_progress_hook, Job, JobManager, JobState};
use godsend_core::{doctor, Error, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State};

/// Shared app state.
pub struct AppState {
    jobs: JobManager,
    config: Mutex<Config>,
    /// Volume mount the UI is currently working against.
    active: Mutex<Option<String>>,
}

impl AppState {
    fn new() -> Self {
        let jobs = JobManager::new();
        jobs.load_journal();
        Self {
            jobs,
            config: Mutex::new(Config::load()),
            active: Mutex::new(None),
        }
    }

    fn paths(&self) -> Result<(Volume, PspPaths)> {
        if let Some(mount) = self.active.lock().expect("active lock").clone() {
            let paths = PspPaths::new(&mount);
            if paths.validate().is_ok() {
                let volume = device::scan_volumes()
                    .into_iter()
                    .find(|v| v.mount == mount)
                    .unwrap_or(Volume {
                        mount: mount.clone(),
                        label: mount.clone(),
                        is_psp: true,
                        free_bytes: 0,
                        total_bytes: 0,
                    });
                return Ok((volume, paths));
            }
            tracing::warn!("active volume {} disappeared, falling back to scan", mount);
        }
        device::require_psp()
    }

    fn config(&self) -> Config {
        self.config.lock().expect("config lock").clone()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub volumes: Vec<Volume>,
    pub active: Option<String>,
    pub config: Config,
    pub doctor: Vec<doctor::Check>,
    pub jobs: Vec<Job>,
}

#[tauri::command]
fn snapshot(state: State<'_, AppState>) -> Snapshot {
    let volumes = device::scan_volumes();
    let active = state.active.lock().expect("active lock").clone();
    Snapshot {
        volumes,
        active,
        config: state.config(),
        doctor: doctor::run(&state.config()),
        jobs: state.jobs.list(),
    }
}

#[tauri::command]
fn set_active_device(state: State<'_, AppState>, mount: String) -> Result<()> {
    let paths = PspPaths::new(&mount);
    paths.validate()?;
    *state.active.lock().expect("active lock") = Some(mount);
    Ok(())
}

#[tauri::command]
fn rescan() -> Vec<Volume> {
    device::scan_volumes()
}

#[tauri::command]
fn eject_device(mount: String) -> Result<()> {
    eject(&mount)
}

#[derive(Serialize)]
struct LibraryView {
    games: Vec<GameEntry>,
    images: Vec<DiscImage>,
}

#[tauri::command]
fn library_scan(state: State<'_, AppState>) -> Result<LibraryView> {
    let (_, paths) = state.paths()?;
    let lib = library::scan(&paths)?;
    Ok(LibraryView {
        games: lib.games,
        images: lib.images,
    })
}

/// ICON0.PNG as a base64 data URL (cached on disk by the core).
#[tauri::command]
fn icon(state: State<'_, AppState>, id: String) -> Option<String> {
    let paths = state.paths().ok()?.1;
    let bytes = library::icon_bytes(&paths, &id)?;
    let url = format!("data:image/png;base64,{}", base64_encode(&bytes));
    Some(url)
}

#[tauri::command]
fn game_delete(state: State<'_, AppState>, id: String) -> Result<()> {
    let (_, paths) = state.paths()?;
    library::delete_entry(&paths, &id)
}

#[tauri::command]
fn homebrew_catalog(state: State<'_, AppState>) -> Result<Vec<CatalogEntry>> {
    let _ = state;
    Ok(homebrew::load_catalog())
}

#[tauri::command]
fn homebrew_refresh(_state: State<'_, AppState>) -> Result<Vec<CatalogEntry>> {
    let mut catalog = homebrew::load_catalog();
    homebrew::refresh_catalog(&mut catalog)?;
    Ok(catalog)
}

#[tauri::command]
fn homebrew_installed(state: State<'_, AppState>) -> Result<Vec<InstalledEntry>> {
    let (_, paths) = state.paths()?;
    let catalog = homebrew::load_catalog();
    Ok(catalog
        .iter()
        .map(|entry| InstalledEntry {
            id: entry.id.clone(),
            installed: homebrew::is_installed(entry, &paths),
        })
        .collect())
}

#[derive(Serialize)]
pub struct InstalledEntry {
    pub id: String,
    pub installed: bool,
}

#[tauri::command]
fn homebrew_install(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<u64> {
    let (volume, paths) = state.paths()?;
    let mut catalog = homebrew::load_catalog();
    let entry = catalog
        .iter_mut()
        .find(|e| e.id == id)
        .cloned()
        .ok_or_else(|| Error::NotFound(format!("catalog entry {}", id)))?;
    let jobs = &state.jobs;
    let app2 = app.clone();
    let job_id = jobs.spawn(
        "install",
        &format!("{} {}", entry.name, entry.latest_tag.clone().unwrap_or_default()),
        move |job| {
            let _ = app2.emit("job://done", job);
        },
        move |ctx| {
            let report = homebrew::install_with_job(&entry, &paths, ctx)?;
            ctx.message(format!(
                "installed {} item(s) onto {}",
                report.installed.len(),
                volume.label
            ));
            Ok(())
        },
    );
    Ok(job_id)
}

#[tauri::command]
fn homebrew_import(app: AppHandle, state: State<'_, AppState>, zip: String) -> Result<u64> {
    let (_, paths) = state.paths()?;
    let zip_path = PathBuf::from(&zip);
    let name = zip_path
        .file_stem()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "homebrew".into());
    let app2 = app.clone();
    let job_id = state.jobs.spawn(
        "install",
        &format!("import {}", name),
        move |job| {
            let _ = app2.emit("job://done", job);
        },
        move |ctx| {
            let report: InstallReport = homebrew::import_zip(&zip_path, &paths, Some(ctx))?;
            ctx.message(format!("installed {} item(s)", report.installed.len()));
            Ok(())
        },
    );
    Ok(job_id)
}

#[tauri::command]
fn cfw_detect(state: State<'_, AppState>) -> Result<CfwState> {
    let (_, paths) = state.paths()?;
    Ok(cfw::detect(&paths))
}

#[tauri::command]
fn cfw_plan(state: State<'_, AppState>) -> Result<Vec<String>> {
    let (_, paths) = state.paths()?;
    Ok(cfw::migration_steps(&cfw::detect(&paths)))
}

#[derive(Debug, Clone, Serialize)]
pub struct MediaFile {
    /// Path relative to the media root (artist/album/track.mp3 style).
    pub rel: String,
    pub name: String,
    pub size_bytes: u64,
}

fn list_media(root: &Path) -> Result<Vec<MediaFile>> {
    let mut out = Vec::new();
    if !root.is_dir() {
        return Ok(out);
    }
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)?.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || name.starts_with("._") {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
            } else {
                let rel = path
                    .strip_prefix(root)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or(name.clone());
                out.push(MediaFile {
                    rel,
                    name,
                    size_bytes: entry.metadata().map(|m| m.len()).unwrap_or(0),
                });
            }
        }
    }
    out.sort_by(|a, b| a.rel.cmp(&b.rel));
    Ok(out)
}

#[tauri::command]
fn music_list(state: State<'_, AppState>) -> Result<Vec<MediaFile>> {
    let (_, paths) = state.paths()?;
    list_media(&paths.music())
}

#[tauri::command]
fn video_list(state: State<'_, AppState>) -> Result<Vec<MediaFile>> {
    let (_, paths) = state.paths()?;
    list_media(&paths.video())
}

/// Short muted preview clip for a video on the stick (PSP-style hover).
#[tauri::command]
fn video_preview(state: State<'_, AppState>, rel: String) -> Result<Option<String>> {
    let (_, paths) = state.paths()?;
    let config = state.config();
    let root = paths.video();
    let src = root.join(&rel);
    if !src.starts_with(&root) || !src.is_file() {
        return Err(Error::NotFound(rel));
    }
    let Some(ffmpeg) = media::find_ffmpeg(&config.ffmpeg_path) else {
        return Ok(None);
    };
    let clip = media::preview_clip(
        &ffmpeg,
        &src,
        &godsend_core::config::cache_dir().join("previews"),
        3.0,
    )?;
    let bytes = std::fs::read(clip)?;
    Ok(Some(format!("data:video/mp4;base64,{}", base64_encode(&bytes))))
}

#[tauri::command]
fn media_delete(state: State<'_, AppState>, kind: String, rel: String) -> Result<()> {
    let (_, paths) = state.paths()?;
    let root = if kind == "video" { paths.video() } else { paths.music() };
    let target = root.join(&rel);
    if !target.starts_with(&root) || !target.is_file() {
        return Err(Error::NotFound(rel));
    }
    std::fs::remove_file(target)?;
    Ok(())
}

#[tauri::command]
fn music_push(app: AppHandle, state: State<'_, AppState>, files: Vec<String>, convert: bool) -> Result<Vec<u64>> {
    let (volume, paths) = state.paths()?;
    let config = state.config();
    let ffmpeg = media::find_ffmpeg(&config.ffmpeg_path);
    let ffprobe = media::find_ffprobe(&config.ffprobe_path);
    let format = if config.music_format == "m4a" {
        media::AudioFormat::M4a
    } else {
        media::AudioFormat::Mp3
    };
    let dest_root = paths.music();
    std::fs::create_dir_all(&dest_root)?;

    let mut ids = Vec::new();
    for file in files {
        let src = PathBuf::from(&file);
        let name = media::fat_safe_name(
            &src.file_stem()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "track".into()),
        );
        let raw = !convert || media::is_psp_audio(&src) || ffmpeg.is_none();
        let label = src
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| name.clone());
        let ffmpeg_clone = ffmpeg.clone();
        let ffprobe_clone = ffprobe.clone();
        let dest_root_clone = dest_root.clone();
        let volume_label = volume.label.clone();
        let app2 = app.clone();
        let job_id = state.jobs.spawn(
            "music",
            &label,
            move |job| {
                let _ = app2.emit("job://done", job);
            },
            move |ctx| {
                ctx.check_cancel()?;
                let dest = if raw {
                    let ext = src
                        .extension()
                        .map(|e| e.to_string_lossy().to_string())
                        .unwrap_or_else(|| "mp3".into());
                    dest_root_clone.join(format!("{}.{}", name, ext))
                } else {
                    dest_root_clone.join(format!("{}.{}", name, format.ext()))
                };
                if raw {
                    ctx.message("copying");
                    copy_sized(&src, &dest, ctx)?;
                } else {
                    ctx.message("converting");
                    media::convert_audio(
                        ffmpeg_clone.as_ref().expect("ffmpeg checked"),
                        ffprobe_clone.as_deref(),
                        &src,
                        &dest,
                        &format,
                        &cancel_flag(ctx.job_id()),
                        &mut |done, total| ctx.progress(done, total),
                    )?;
                }
                ctx.message(format!("on {} as {}", volume_label, dest.file_name().unwrap_or_default().to_string_lossy()));
                Ok(())
            },
        );
        ids.push(job_id);
    }
    Ok(ids)
}

#[tauri::command]
fn video_push(
    app: AppHandle,
    state: State<'_, AppState>,
    files: Vec<String>,
    convert: bool,
    thm: bool,
) -> Result<Vec<u64>> {
    let (volume, paths) = state.paths()?;
    let config = state.config();
    let ffmpeg = media::find_ffmpeg(&config.ffmpeg_path);
    let ffprobe = media::find_ffprobe(&config.ffprobe_path);
    let make_thm = thm && config.video_thumbnails;
    let quality = media::VideoQuality {
        preset: config.video_preset.clone(),
        crf: config.video_crf,
        fps: config.video_fps,
        audio_kbps: config.video_audio_kbps,
        resolution: config.video_resolution.clone(),
    };
    let dest_root = paths.video();
    std::fs::create_dir_all(&dest_root)?;

    let mut ids = Vec::new();
    for file in files {
        let src = PathBuf::from(&file);
        let name = media::fat_safe_name(
            &src.file_stem()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "video".into()),
        );
        let raw = !convert || media::is_psp_video(&src) || ffmpeg.is_none();
        let label = src
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| name.clone());
        let ffmpeg_clone = ffmpeg.clone();
        let ffprobe_clone = ffprobe.clone();
        let dest_root_clone = dest_root.clone();
        let volume_label = volume.label.clone();
        let quality_clone = quality.clone();
        let app2 = app.clone();
        let job_id = state.jobs.spawn(
            "video",
            &label,
            move |job| {
                let _ = app2.emit("job://done", job);
            },
            move |ctx| {
                ctx.check_cancel()?;
                let dest = dest_root_clone.join(format!("{}.mp4", name));
                if raw {
                    ctx.message("copying");
                    copy_sized(&src, &dest, ctx)?;
                } else {
                    ctx.message("converting to PSP AVC");
                    let thm_dest = if make_thm {
                        Some(dest_root_clone.join(format!("{}.THM", name)))
                    } else {
                        None
                    };
                    media::convert_video(
                        ffmpeg_clone.as_ref().expect("ffmpeg checked"),
                        ffprobe_clone.as_deref(),
                        &src,
                        &dest,
                        thm_dest.as_deref(),
                        &quality_clone,
                        &cancel_flag(ctx.job_id()),
                        &mut |done, total| ctx.progress(done, total),
                    )?;
                }
                ctx.message(format!("on {} as {}", volume_label, dest.file_name().unwrap_or_default().to_string_lossy()));
                Ok(())
            },
        );
        ids.push(job_id);
    }
    Ok(ids)
}

#[tauri::command]
fn iso_import(app: AppHandle, state: State<'_, AppState>, file: String, to_cso: bool) -> Result<u64> {
    let (volume, paths) = state.paths()?;
    let src = PathBuf::from(&file);
    let name = src
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .ok_or_else(|| Error::Invalid("bad file".into()))?;
    let dest = if to_cso {
        paths.iso().join(name.replace(".iso", ".cso"))
    } else {
        paths.iso().join(name.clone())
    };
    std::fs::create_dir_all(&dest.parent().expect("iso parent"))?;
    let level = state.config().cso_level;
    let app2 = app.clone();
    let volume_label = volume.label;
    let job_id = state.jobs.spawn(
        "iso",
        &format!("{}", name),
        move |job| {
            let _ = app2.emit("job://done", job);
        },
        move |ctx| {
            ctx.check_cancel()?;
            if to_cso {
                ctx.message("compressing to CSO");
                godsend_core::cso::iso_to_cso(&src, &dest, level, &mut |done, total| ctx.progress(done, total), &cancel_flag(ctx.job_id()))?;
            } else {
                ctx.message("copying image");
                copy_sized(&src, &dest, ctx)?;
            }
            ctx.message(format!("on {} as {}", volume_label, dest.file_name().unwrap_or_default().to_string_lossy()));
            Ok(())
        },
    );
    Ok(job_id)
}

#[tauri::command]
fn iso_convert(app: AppHandle, state: State<'_, AppState>, image_id: String) -> Result<u64> {
    let (volume, paths) = state.paths()?;
    let file = paths.iso().join(
        image_id
            .strip_prefix("iso/")
            .ok_or_else(|| Error::Invalid("bad image id".into()))?,
    );
    let ext = file
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if ext != "iso" && ext != "cso" {
        return Err(Error::Invalid("only iso/cso can be converted".into()));
    }
    // Content decides the direction, not the extension.
    let already_cso = godsend_core::cso::probe(&file)
        .map(|i| i.is_cso)
        .unwrap_or(false);
    let (mut dest, to_cso) = if already_cso {
        (file.with_extension("iso"), false)
    } else {
        (file.with_extension("cso"), true)
    };
    if dest == file {
        if already_cso {
            // A CSO behind a .iso name: keep both files.
            let stem = file
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            dest = file.with_file_name(format!("{stem}.dec.iso"));
        } else {
            return Err(Error::Invalid(
                "destination would overwrite the source".into(),
            ));
        }
    }
    let level = state.config().cso_level;
    let app2 = app.clone();
    let volume_label = volume.label;
    let job_id = state.jobs.spawn(
        "convert",
        &format!("{}", file.file_name().unwrap_or_default().to_string_lossy()),
        move |job| {
            let _ = app2.emit("job://done", job);
        },
        move |ctx| {
            ctx.check_cancel()?;
            if to_cso {
                ctx.message("compressing to CSO");
                godsend_core::cso::iso_to_cso(&file, &dest, level, &mut |done, total| ctx.progress(done, total), &cancel_flag(ctx.job_id()))?;
            } else {
                ctx.message("decompressing to ISO");
                godsend_core::cso::cso_to_iso(&file, &dest, &mut |done, total| ctx.progress(done, total), &cancel_flag(ctx.job_id()))?;
            }
            ctx.message(format!(
                "wrote {} on {}",
                dest.file_name().unwrap_or_default().to_string_lossy(),
                volume_label
            ));
            Ok(())
        },
    );
    Ok(job_id)
}

#[tauri::command]
fn image_delete(state: State<'_, AppState>, image_id: String) -> Result<()> {
    let (_, paths) = state.paths()?;
    library::delete_entry(&paths, &image_id)
}

#[tauri::command]
fn savedata_list(state: State<'_, AppState>) -> Result<Vec<SaveEntry>> {
    let (_, paths) = state.paths()?;
    savedata::list(&paths)
}

#[tauri::command]
fn savedata_icon(state: State<'_, AppState>, folder: String) -> Option<String> {
    let paths = state.paths().ok()?.1;
    let bytes = savedata::icon_bytes(&paths, &folder)?;
    Some(format!("data:image/png;base64,{}", base64_encode(&bytes)))
}

#[tauri::command]
fn savedata_backup(app: AppHandle, state: State<'_, AppState>, folders: Vec<String>) -> Result<u64> {
    let (volume, paths) = state.paths()?;
    let stamp = chrono_stamp();
    let dest = godsend_core::config::data_dir().join(format!("saves-backup-{}.zip", stamp));
    let app2 = app.clone();
    let job_id = state.jobs.spawn(
        "backup",
        &format!("saves -> {}", dest.file_name().unwrap_or_default().to_string_lossy()),
        move |job| {
            let _ = app2.emit("job://done", job);
        },
        move |ctx| {
            let written = savedata::backup(&paths, &folders, &dest, Some(ctx))?;
            ctx.message(format!("{} bytes zipped ({} volume)", written, volume.label));
            Ok(())
        },
    );
    Ok(job_id)
}

#[tauri::command]
fn savedata_restore(app: AppHandle, state: State<'_, AppState>, zip: String) -> Result<u64> {
    let (_, paths) = state.paths()?;
    let zip_path = PathBuf::from(&zip);
    let app2 = app.clone();
    let job_id = state.jobs.spawn(
        "restore",
        &format!("restore {}", zip_path.file_name().unwrap_or_default().to_string_lossy()),
        move |job| {
            let _ = app2.emit("job://done", job);
        },
        move |ctx| {
            let files = savedata::restore(&paths, &zip_path, Some(ctx))?;
            ctx.message(format!("restored {} files", files));
            Ok(())
        },
    );
    Ok(job_id)
}

#[tauri::command]
fn savedata_delete(state: State<'_, AppState>, folder: String) -> Result<()> {
    let (_, paths) = state.paths()?;
    savedata::delete_save(&paths, &folder)
}

#[tauri::command]
fn savedata_reveal(state: State<'_, AppState>, folder: String) -> Result<String> {
    let (_, paths) = state.paths()?;
    let dir = paths.savedata().join(&folder);
    if !dir.is_dir() {
        return Err(Error::NotFound(folder));
    }
    Ok(dir.to_string_lossy().to_string())
}

#[derive(Serialize)]
struct PluginsView {
    game: Vec<PluginEntry>,
    vsh: Vec<PluginEntry>,
    pops: Vec<PluginEntry>,
}

#[tauri::command]
fn plugins_list(state: State<'_, AppState>) -> Result<PluginsView> {
    let (_, paths) = state.paths()?;
    Ok(PluginsView {
        game: plugins::list(&paths, Section::Game)?,
        vsh: plugins::list(&paths, Section::Vsh)?,
        pops: plugins::list(&paths, Section::Pops)?,
    })
}

#[tauri::command]
fn plugin_toggle(state: State<'_, AppState>, section: String, path: String, enable: bool) -> Result<()> {
    let (_, paths) = state.paths()?;
    let section = Section::parse(&section).ok_or_else(|| Error::Invalid(format!("unknown section {}", section)))?;
    plugins::toggle(&paths, section, &path, enable)
}

#[tauri::command]
fn plugin_add(state: State<'_, AppState>, section: String, prx: String) -> Result<PluginEntry> {
    let (_, paths) = state.paths()?;
    let section = Section::parse(&section).ok_or_else(|| Error::Invalid(format!("unknown section {}", section)))?;
    plugins::add(&paths, section, Path::new(&prx))
}

#[tauri::command]
fn plugin_remove(state: State<'_, AppState>, section: String, path: String) -> Result<()> {
    let (_, paths) = state.paths()?;
    let section = Section::parse(&section).ok_or_else(|| Error::Invalid(format!("unknown section {}", section)))?;
    plugins::remove(&paths, section, &path)
}

#[tauri::command]
fn themes_list(state: State<'_, AppState>) -> Result<Vec<ThemeEntry>> {
    let (_, paths) = state.paths()?;
    themes::list(&paths)
}

#[tauri::command]
fn themes_remote(state: State<'_, AppState>) -> Result<Vec<themes::RemoteTheme>> {
    let config = state.config();
    themes::remote_list(&config)
}

#[tauri::command]
fn theme_install_remote(
    state: State<'_, AppState>,
    name: String,
    url: String,
    format: Option<String>,
) -> Result<ThemeEntry> {
    let (_, paths) = state.paths()?;
    let config = state.config();
    let theme = themes::RemoteTheme {
        name,
        url,
        format: format.unwrap_or_default(),
        size_bytes: 0,
    };
    themes::install_remote(&config, &theme, &paths)
}

#[tauri::command]
fn theme_install(state: State<'_, AppState>, file: String) -> Result<ThemeEntry> {
    let (_, paths) = state.paths()?;
    themes::install(&paths, Path::new(&file))
}

#[tauri::command]
fn theme_delete(state: State<'_, AppState>, name: String) -> Result<()> {
    let (_, paths) = state.paths()?;
    themes::delete(&paths, &name)
}

// ---------- Theme Studio packs (wallpaper packs + app accent) ----------

#[tauri::command]
fn theme_pack_create(
    state: State<'_, AppState>,
    name: String,
    accent: String,
    wallpaper: Option<String>,
) -> Result<ThemePack> {
    let (_, paths) = state.paths()?;
    // The zip is a by-product kept in the app cache; the pack itself is written
    // straight into PSP/PICTURE/<slug>/ so it is usable immediately.
    let out_dir = godsend_core::config::cache_dir().join("theme-packs");
    let wallpaper_path = wallpaper.as_deref().map(Path::new);
    let zip = theme_pack::create(&name, &accent, wallpaper_path, &out_dir)?;
    theme_pack::install(&paths, &zip)
}

#[tauri::command]
fn theme_packs_list(state: State<'_, AppState>) -> Result<Vec<ThemePack>> {
    let (_, paths) = state.paths()?;
    theme_pack::list(&paths)
}

#[tauri::command]
fn theme_pack_install(state: State<'_, AppState>, zip: String) -> Result<ThemePack> {
    let (_, paths) = state.paths()?;
    theme_pack::install(&paths, Path::new(&zip))
}

#[tauri::command]
fn theme_pack_delete(state: State<'_, AppState>, folder: String) -> Result<()> {
    let (_, paths) = state.paths()?;
    theme_pack::delete(&paths, &folder)
}

#[tauri::command]
fn doctor_run(state: State<'_, AppState>) -> Vec<doctor::Check> {
    doctor::run(&state.config())
}

#[tauri::command]
fn config_get(state: State<'_, AppState>) -> Config {
    state.config()
}

#[tauri::command]
fn config_set(state: State<'_, AppState>, patch: serde_json::Value) -> Result<Config> {
    let mut config = state.config.lock().expect("config lock");
    config.patch_from_json(&patch)?;
    Ok(config.clone())
}

#[tauri::command]
fn jobs(state: State<'_, AppState>) -> Vec<Job> {
    state.jobs.list()
}

#[tauri::command]
fn job_cancel(state: State<'_, AppState>, id: u64) -> bool {
    state.jobs.cancel(id)
}

#[tauri::command]
fn jobs_clear(state: State<'_, AppState>) {
    state.jobs.clear_finished();
}

#[tauri::command]
fn reveal(path: String) -> Result<()> {
    reveal_in_finder(Path::new(&path))
}

#[tauri::command]
fn open_data_dir() -> Result<String> {
    let dir = godsend_core::config::data_dir();
    std::fs::create_dir_all(&dir)?;
    reveal_in_finder(&dir)?;
    Ok(dir.to_string_lossy().to_string())
}

// ---------- store / downloads / debrid / files ----------

#[tauri::command]
fn debrid_status(state: State<'_, AppState>) -> godsend_core::debrid::DebridStatus {
    let config = state.config();
    godsend_core::debrid::status(&config.rd_api_key, &config.tb_api_key)
}

#[tauri::command]
fn store_search(state: State<'_, AppState>, query: String, source: String, limit: u32) -> Result<serde_json::Value> {
    let config = state.config();
    match source.as_str() {
        "ia" => {
            let q = if query.is_empty() {
                godsend_core::sources::ia_seed_queries()[0].to_string()
            } else {
                query
            };
            let items = godsend_core::sources::ia_search(&q, 40)?;
            Ok(serde_json::json!({ "kind": "ia", "items": items }))
        }
        "minerva" => {
            let sets = godsend_core::sources::minerva_sets()?;
            Ok(serde_json::json!({ "kind": "minerva", "items": sets }))
        }
        "rroms" => {
            let items = godsend_core::sources::rroms_items()?;
            Ok(serde_json::json!({ "kind": "rroms", "items": items }))
        }
        "nps" => {
            let entries = godsend_core::sources::nps_search(&query, limit.max(1).min(120))?;
            Ok(serde_json::json!({ "kind": "nps", "items": entries }))
        }
        _ => {
            let hits = godsend_core::sources::gamesearch(&config.gamesearch_base, &query, limit.max(1).min(60))?;
            // record into the catalog db (PPSSPP-style)
            let games = hits
                .iter()
                .map(|h| godsend_core::catalog::CatalogGame {
                    serial: String::new(),
                    title: h.title.clone(),
                    region: String::new(),
                    size_bytes: h.size.unwrap_or(0),
                    cover: h.cover.clone().unwrap_or_default(),
                    animation: String::new(),
                    source: h.source.clone(),
                    slug: h.slug.clone(),
                })
                .collect();
            let _ = godsend_core::catalog::upsert(games);
            Ok(serde_json::json!({ "kind": "gamesearch", "items": hits }))
        }
    }
}

#[tauri::command]
fn store_detail(state: State<'_, AppState>, source: String, id: String) -> Result<serde_json::Value> {
    let config = state.config();
    match source.as_str() {
        "ia" => {
            let files = godsend_core::sources::ia_item_files(&id)?;
            Ok(serde_json::json!({ "kind": "ia", "files": files }))
        }
        "minerva" => Ok(serde_json::json!({ "kind": "minerva", "files": [] })),
        "rroms" => {
            let files = godsend_core::sources::rroms_files(&id)?;
            Ok(serde_json::json!({ "kind": "rroms", "files": files }))
        }
        _ => {
            let hit = godsend_core::sources::gamesearch_detail(&config.gamesearch_base, &id)?;
            Ok(serde_json::json!({ "kind": "gamesearch", "hit": hit }))
        }
    }
}

#[tauri::command]
fn store_download(app: AppHandle, state: State<'_, AppState>, source: String, id: String) -> Result<u64> {
    let config = state.config();
    let app2 = app.clone();
    let job_id = match source.as_str() {
        "minerva" => {
            let url = godsend_core::sources::minerva_download(&id)
                .ok_or_else(|| Error::Invalid("bad minerva id".into()))?;
            let label = id.clone();
            let dest_dir = godsend_core::aria2::download_dir(&config);
            state.jobs.spawn(
                "download",
                &format!("torrent {label}"),
                move |job| {
                    let _ = app2.emit("job://done", job);
                },
                move |ctx| {
                    ctx.check_cancel()?;
                    let path = godsend_core::aria2::download(&url, &dest_dir, &label, &config, ctx)?;
                    ctx.message(format!("torrent saved to {}", path.display()));
                    Ok(())
                },
            )
        }
        "ia" => {
            // id = "<item>/<file name>"; resolve to a direct IA url
            let (item, file) = id.split_once('/').ok_or_else(|| Error::Invalid("bad ia id".into()))?;
            let files = godsend_core::sources::ia_item_files(item)?;
            let hit = files
                .iter()
                .find(|f| f.name == file)
                .ok_or_else(|| Error::NotFound(file.to_string()))?;
            let url = godsend_core::sources::ia_signed_url(&hit.url, &config);
            let label = file.to_string();
            let dest_dir = godsend_core::aria2::download_dir(&config);
            state.jobs.spawn(
                "download",
                &label.clone(),
                move |job| {
                    let _ = app2.emit("job://done", job);
                },
                move |ctx| {
                    ctx.check_cancel()?;
                    let path = godsend_core::aria2::download(&url, &dest_dir, &label, &config, ctx)
                        .map_err(|err| Error::Other(config.redact(&err.to_string())))?;
                    ctx.message(format!("downloaded {}", path.display()));
                    Ok(())
                },
            )
        }
        "rroms" => {
            // id is an r-roms item; archive.org items expand to their files.
            let files = godsend_core::sources::rroms_files(&id)?;
            let label = format!("r-roms {id}");
            let count = files.len();
            let dest_dir = godsend_core::aria2::download_dir(&config);
            state.jobs.spawn(
                "download",
                &label,
                move |job| {
                    let _ = app2.emit("job://done", job);
                },
                move |ctx| {
                    for (index, file) in files.iter().enumerate() {
                        ctx.check_cancel()?;
                        if count > 1 {
                            ctx.message(format!("file {}/{}: {}", index + 1, count, file.name));
                        }
                        let name = file
                            .name
                            .rsplit('/')
                            .next()
                            .filter(|name| !name.is_empty())
                            .unwrap_or(&id);
                        let name = godsend_core::media::fat_safe_name(name);
                        let url = godsend_core::sources::ia_signed_url(&file.url, &config);
                        godsend_core::aria2::download(&url, &dest_dir, &name, &config, ctx)
                            .map_err(|err| Error::Other(config.redact(&err.to_string())))?;
                    }
                    ctx.message(format!("downloaded {count} file(s) to {}", dest_dir.display()));
                    Ok(())
                },
            )
        }
        "nps" => {
            let entry = godsend_core::sources::nps_entry(&id)?;
            let label = format!("{} [{}]", entry.title, entry.title_id);
            let name = godsend_core::sources::nps_pkg_name(&entry);
            let dest_dir = godsend_core::aria2::download_dir(&config);
            state.jobs.spawn(
                "download",
                &label,
                move |job| {
                    let _ = app2.emit("job://done", job);
                },
                move |ctx| {
                    ctx.check_cancel()?;
                    let path =
                        godsend_core::aria2::download(&entry.url, &dest_dir, &name, &config, ctx)?;
                    // pkg2zip and NoPayStation want the key beside the package.
                    let stem = path
                        .file_stem()
                        .map(|stem| stem.to_string_lossy().to_string())
                        .unwrap_or_else(|| name.clone());
                    let sidecar = path.with_file_name(format!("{stem}.zrif.txt"));
                    let body = match entry.zrif.as_deref() {
                        Some(key) if !key.is_empty() => format!("{key}\n"),
                        _ => "# no zRIF in the NoPayStation index; pkg2zip needs a key\n".to_string(),
                    };
                    std::fs::write(&sidecar, body)?;
                    ctx.message(format!("downloaded {}", path.display()));
                    Ok(())
                },
            )
        }
        _ => {
            let hit = godsend_core::sources::gamesearch_detail(&config.gamesearch_base, &id)?;
            let label = hit.title.clone();
            state.jobs.spawn(
                "download",
                &label,
                move |job| {
                    let _ = app2.emit("job://done", job);
                },
                move |ctx| godsend_core::store::download_game(hit, config, ctx).map(|_| ()),
            )
        }
    };
    Ok(job_id)
}

#[derive(Serialize)]
struct DownloadsView {
    dir: String,
    files: Vec<godsend_core::fm::FmEntry>,
}

#[tauri::command]
fn downloads_list(state: State<'_, AppState>) -> Result<DownloadsView> {
    let config = state.config();
    let dir = godsend_core::aria2::download_dir(&config);
    let files = godsend_core::fm::list_dir(&dir).unwrap_or_default();
    Ok(DownloadsView {
        dir: dir.to_string_lossy().to_string(),
        files,
    })
}

#[tauri::command]
fn download_dir_set(state: State<'_, AppState>, path: String) -> Result<()> {
    let dir = PathBuf::from(&path);
    if !dir.is_dir() {
        return Err(Error::NotFound(path));
    }
    state
        .config
        .lock()
        .expect("config lock")
        .patch_from_json(&serde_json::json!({ "download_dir": path }))?;
    Ok(())
}

#[tauri::command]
fn download_open(state: State<'_, AppState>, name: String) -> Result<String> {
    let config = state.config();
    let dir = godsend_core::aria2::download_dir(&config);
    let path = dir.join(&name);
    if !path.exists() {
        return Err(Error::NotFound(name));
    }
    reveal_in_finder(&path)?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
fn download_delete(state: State<'_, AppState>, name: String) -> Result<()> {
    let config = state.config();
    let dir = godsend_core::aria2::download_dir(&config);
    godsend_core::fm::delete_local(&dir.join(&name), &dir)
}

#[tauri::command]
fn download_push(app: AppHandle, state: State<'_, AppState>, name: String) -> Result<u64> {
    let (volume, paths) = state.paths()?;
    let config = state.config();
    let local = godsend_core::aria2::download_dir(&config).join(&name);
    if !local.exists() {
        return Err(Error::NotFound(name));
    }
    let app2 = app.clone();
    let volume_label = volume.label.clone();
    let job_id = state.jobs.spawn(
        "copy",
        &format!("{} -> PSP", name),
        move |job| {
            let _ = app2.emit("job://done", job);
        },
        move |ctx| {
            let dest = godsend_core::store::push_local_to_psp(&local, &paths, ctx)?;
            ctx.message(format!("on {} as {}", volume_label, dest.display()));
            Ok(())
        },
    );
    Ok(job_id)
}

// ---------- file manager + libraries ----------

#[tauri::command]
fn fm_list(path: String) -> Result<Vec<godsend_core::fm::FmEntry>> {
    godsend_core::fm::list_dir(Path::new(&path))
}

#[tauri::command]
fn fm_mkdir(path: String, name: String) -> Result<String> {
    let dir = godsend_core::fm::mkdir(Path::new(&path), &name)?;
    Ok(dir.to_string_lossy().to_string())
}

#[tauri::command]
fn fm_delete(state: State<'_, AppState>, path: String) -> Result<()> {
    // Allowed roots: the download dir and the PSP volume.
    let config = state.config();
    let dl = godsend_core::aria2::download_dir(&config);
    if Path::new(&path).starts_with(&dl) {
        return godsend_core::fm::delete_local(Path::new(&path), &dl);
    }
    if let Ok((_, paths)) = state.paths() {
        if Path::new(&path).starts_with(&paths.root) {
            return godsend_core::fm::delete_local(Path::new(&path), &paths.root);
        }
    }
    Err(Error::Invalid("path outside managed roots".into()))
}

#[tauri::command]
fn fm_to_psp(app: AppHandle, state: State<'_, AppState>, path: String, dest_kind: String) -> Result<u64> {
    let (_, paths) = state.paths()?;
    let local = PathBuf::from(&path);
    if !local.exists() {
        return Err(Error::NotFound(path));
    }
    let app2 = app.clone();
    let job_id = state.jobs.spawn(
        "copy",
        &format!("{} -> PSP", local.file_name().unwrap_or_default().to_string_lossy()),
        move |job| {
            let _ = app2.emit("job://done", job);
        },
        move |ctx| {
            let dest = godsend_core::fm::copy_to_psp(&local, &paths, &dest_kind, ctx)?;
            ctx.message(format!("on stick at {}", dest.display()));
            Ok(())
        },
    );
    Ok(job_id)
}

#[tauri::command]
fn fm_from_psp(app: AppHandle, state: State<'_, AppState>, psp_id: String) -> Result<u64> {
    let (_, paths) = state.paths()?;
    let config = state.config();
    let local_root = godsend_core::aria2::download_dir(&config);
    let app2 = app.clone();
    let job_id = state.jobs.spawn(
        "copy",
        &format!("{} -> local", psp_id),
        move |job| {
            let _ = app2.emit("job://done", job);
        },
        move |ctx| {
            let dest = godsend_core::fm::copy_from_psp(&psp_id, &paths, &local_root, ctx)?;
            ctx.message(format!("local at {}", dest.display()));
            Ok(())
        },
    );
    Ok(job_id)
}

#[tauri::command]
fn local_library(state: State<'_, AppState>) -> Result<Vec<godsend_core::fm::LocalGame>> {
    let config = state.config();
    godsend_core::fm::local_library(&godsend_core::aria2::download_dir(&config))
}

// ---------- catalog covers / info / mods ----------

#[tauri::command]
fn game_info(state: State<'_, AppState>, serial: String, title: String) -> Result<serde_json::Value> {
    let config = state.config();
    let known = godsend_core::catalog::lookup_serial(&serial)
        .or_else(|| godsend_core::catalog::lookup_best(&title));
    let mods = godsend_core::mods::list(&config, &serial).unwrap_or_default();
    let animation = godsend_core::catalog::animation_url(&config, &serial)
        .or_else(|| known.as_ref().map(|k| k.animation.clone()).filter(|a| !a.is_empty()));
    Ok(serde_json::json!({
        "known": known,
        "animation": animation,
        "mods": mods,
    }))
}

#[tauri::command]
fn cover(
    state: State<'_, AppState>,
    id: String,
    serial: Option<String>,
    title: Option<String>,
) -> Option<String> {
    let paths = state.paths().ok()?.1;
    let config = state.config();
    // 1. art straight from the game (ICON0 next to EBOOT, PBP, or ISO).
    if let Some(bytes) = library::asset_bytes(&paths, &id, "ICON0.PNG") {
        return Some(format!("data:image/png;base64,{}", base64_encode(&bytes)));
    }
    // 2. R2/catalog cover by serial, 3. remote index cover, 4. stick ICON0.
    let serial_value = serial.clone().unwrap_or_default();
    if !serial_value.is_empty() {
        let fallback = godsend_core::catalog::lookup_serial(&serial_value)
            .map(|g| g.cover)
            .unwrap_or_default();
        if let Some(bytes) = godsend_core::catalog::cover_bytes(&config, &serial_value, &fallback) {
            return Some(format!("data:image/png;base64,{}", base64_encode(&bytes)));
        }
    }
    // 3. remote cover from the store index (cached on disk), 4. stick ICON0.
    let title_value = title.unwrap_or_default();
    if let Some(bytes) = godsend_core::catalog::remote_cover(&config, &serial_value, &title_value) {
        return Some(format!("data:image/png;base64,{}", base64_encode(&bytes)));
    }
    let bytes = library::icon_bytes(&paths, &id)?;
    Some(format!("data:image/png;base64,{}", base64_encode(&bytes)))
}

/// The game's own animated icon, transcoded for the webview.
#[tauri::command]
fn animation(state: State<'_, AppState>, id: String) -> Result<Option<String>> {
    let (_, paths) = state.paths()?;
    let config = state.config();
    let Some(pmf) = library::animation_pmf(&paths, &id) else {
        return Ok(None);
    };
    let Some(ffmpeg) = media::find_ffmpeg(&config.ffmpeg_path) else {
        return Ok(None);
    };
    let clip = media::transcode_pmf(
        &ffmpeg,
        &pmf,
        &godsend_core::config::cache_dir().join("animations"),
        &id,
    )?;
    let bytes = std::fs::read(clip)?;
    Ok(Some(format!("data:video/mp4;base64,{}", base64_encode(&bytes))))
}

#[tauri::command]
fn mods_list(state: State<'_, AppState>, serial: String) -> Result<Vec<godsend_core::mods::ModEntry>> {
    let config = state.config();
    godsend_core::mods::list(&config, &serial)
}

#[tauri::command]
fn mods_install(app: AppHandle, state: State<'_, AppState>, serial: String, url: String, name: String) -> Result<u64> {
    let config = state.config();
    let dest_dir = godsend_core::mods::mods_dir(&config, &serial);
    let label = name.clone();
    let app2 = app.clone();
    let job_id = state.jobs.spawn(
        "download",
        &format!("mod {label}"),
        move |job| {
            let _ = app2.emit("job://done", job);
        },
        move |ctx| {
            ctx.check_cancel()?;
            let path = godsend_core::aria2::download(&url, &dest_dir, &label, &config, ctx)?;
            ctx.message(format!("mod downloaded to {}", path.display()));
            Ok(())
        },
    );
    Ok(job_id)
}

/// Cancel flag registry so ffmpeg conversions can observe cancels from the UI.
static CANCELS: std::sync::OnceLock<Mutex<std::collections::HashMap<u64, Arc<AtomicBool>>>> =
    std::sync::OnceLock::new();

fn cancel_flag(job_id: u64) -> Arc<AtomicBool> {
    let lock = CANCELS.get_or_init(|| Mutex::new(std::collections::HashMap::new()));
    let mut map = lock.lock().expect("cancel lock");
    map.entry(job_id)
        .or_insert_with(|| Arc::new(AtomicBool::new(false)))
        .clone()
}

// Prune cancel entries for finished jobs occasionally from job_done handler.

fn copy_sized(src: &Path, dest: &Path, ctx: &godsend_core::transfer::JobContext) -> Result<u64> {
    let total = std::fs::metadata(src).map(|m| m.len()).unwrap_or(0);
    ctx.progress(0, total);
    godsend_core::transfer::copy_with_progress(src, dest, ctx)
}

fn eject(mount: &str) -> Result<()> {
    let output = if cfg!(target_os = "macos") {
        std::process::Command::new("diskutil")
            .args(["eject", mount])
            .output()
    } else if cfg!(target_os = "linux") {
        std::process::Command::new("umount").arg(mount).output()
    } else {
        return Err(Error::Other(
            "safe eject is not supported on Windows; use the tray icon".into(),
        ));
    };
    match output {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => Err(Error::Other(format!(
            "eject failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ))),
        Err(e) => Err(Error::Other(format!("eject failed: {}", e))),
    }
}

fn reveal_in_finder(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(Error::NotFound(path.display().to_string()));
    }
    #[cfg(target_os = "macos")]
    let ok = std::process::Command::new("open").arg(path).spawn().is_ok();
    #[cfg(target_os = "windows")]
    let ok = std::process::Command::new("explorer").arg(path).spawn().is_ok();
    #[cfg(all(unix, not(target_os = "macos")))]
    let ok = std::process::Command::new("xdg-open").arg(path).spawn().is_ok();
    if ok {
        Ok(())
    } else {
        Err(Error::Other("could not open file manager".into()))
    }
}

fn chrono_stamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Compact UTC-ish stamp: YYYYMMDD-HHMMSS without pulling chrono.
    let days = secs / 86400;
    let rem = secs % 86400;
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    // Civil-from-days algorithm (Howard Hinnant).
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mth = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mth <= 2 { y + 1 } else { y };
    format!(
        "{:04}{:02}{:02}-{:02}{:02}{:02}",
        y, mth, d, h, m, s
    )
}

fn base64_encode(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// Poll mounted volumes and emit `device://changed` when the set changes.
fn spawn_device_watcher(app: AppHandle) {
    std::thread::spawn(move || {
        let mut signature = String::new();
        loop {
            let volumes = device::scan_volumes();
            let next: String = volumes
                .iter()
                .map(|v| format!("{}:{}:{};", v.mount, v.is_psp, v.free_bytes / (1024 * 1024)))
                .collect();
            if next != signature {
                signature = next.clone();
                let _ = app.emit("device://changed", &volumes);
            }
            std::thread::sleep(std::time::Duration::from_millis(2500));
        }
    });
}

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "godsend_core=info".into()),
        )
        .init();
    let _ = godsend_core::config::ensure_dirs();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .manage(AppState::new())
        .setup(|app| {
            // Bundled ffmpeg/ffprobe (shipped in the resource dir, per arch).
            if let Ok(res) = app.path().resource_dir() {
                let state = app.state::<AppState>();
                let mut cfg = state.config.lock().expect("config lock");
                if cfg.ffmpeg_path.is_empty() {
                    let p = res.join(if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" });
                    if p.is_file() {
                        cfg.ffmpeg_path = p.to_string_lossy().to_string();
                    }
                }
                if cfg.ffprobe_path.is_empty() {
                    let p = res.join(if cfg!(windows) { "ffprobe.exe" } else { "ffprobe" });
                    if p.is_file() {
                        cfg.ffprobe_path = p.to_string_lossy().to_string();
                    }
                }
            }
            let config = app.state::<AppState>().config();
            if config.mcp_enabled {
                let endpoint = godsend_core::mcp::ServerConfig::from_config(&config).url();
                match godsend_core::mcp::serve(&config) {
                    Ok(_) => tracing::info!("MCP server listening on {}", endpoint),
                    Err(err) => tracing::warn!("MCP server not started on {}: {}", endpoint, err),
                }
            }
            let handle = app.handle().clone();
            set_progress_hook(Some(Arc::new(move |job_id, done, total| {
                let _ = handle.emit(
                    "job://progress",
                    serde_json::json!({ "id": job_id, "done": done, "total": total }),
                );
            })));
            spawn_device_watcher(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            snapshot,
            set_active_device,
            rescan,
            eject_device,
            library_scan,
            icon,
            game_delete,
            homebrew_catalog,
            homebrew_refresh,
            homebrew_installed,
            homebrew_install,
            homebrew_import,
            cfw_detect,
            cfw_plan,
            music_list,
            video_list,
            media_delete,
            video_preview,
            music_push,
            video_push,
            iso_import,
            iso_convert,
            image_delete,
            savedata_list,
            savedata_icon,
            savedata_backup,
            savedata_restore,
            savedata_delete,
            savedata_reveal,
            plugins_list,
            plugin_toggle,
            plugin_add,
            plugin_remove,
            themes_list,
            themes_remote,
            theme_install_remote,
            theme_install,
            theme_delete,
            theme_pack_create,
            theme_packs_list,
            theme_pack_install,
            theme_pack_delete,
            doctor_run,
            config_get,
            config_set,
            jobs,
            job_cancel,
            jobs_clear,
            reveal,
            open_data_dir,
            debrid_status,
            store_search,
            store_detail,
            store_download,
            downloads_list,
            download_dir_set,
            download_open,
            download_delete,
            download_push,
            fm_list,
            fm_mkdir,
            fm_delete,
            fm_to_psp,
            fm_from_psp,
            local_library,
            game_info,
            cover,
            animation,
            mods_list,
            mods_install,
        ])
        .run(tauri::generate_context!())
        .expect("error while running GODsend PSP");
}

// JobState import used by typed events (kept for future filtering).
#[allow(unused)]
fn _keep_job_state(_: JobState) {}
