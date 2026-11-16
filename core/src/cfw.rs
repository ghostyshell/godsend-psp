//! Custom firmware inventory for the stick.
//!
//! Everything here is read-only filesystem detection: CFW lives as plain
//! folders on the memory stick, so the state can be reported while the PSP is
//! in USB mode. No console access is required.

use crate::device::PspPaths;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

/// What custom firmware and loaders the stick currently carries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CfwState {
    /// ARK-4 payload folder (`PSP/SAVEDATA/ARK_01234`).
    pub ark4: bool,
    /// ARK-5 / FasterARK loaders (`FasterARK`, `ARK_05`, `ARK_UPDATE`).
    pub ark5: bool,
    /// PRO CFW updater or FastRecovery loader.
    pub pro: bool,
    /// ME CFW updater.
    pub me: bool,
    /// Time Machine folder (`TM/`).
    pub time_machine: bool,
    /// DC10 / ARK_DC recovery folders.
    pub recovery: bool,
    /// ISO driver folder present and carrying at least one image.
    pub iso_driver: bool,
    /// Plugin paths listed in `seplugins/*.txt`, deduplicated.
    pub plugins: Vec<String>,
}

impl CfwState {
    fn empty() -> Self {
        Self {
            ark4: false,
            ark5: false,
            pro: false,
            me: false,
            time_machine: false,
            recovery: false,
            iso_driver: false,
            plugins: Vec::new(),
        }
    }
}

/// Scan the stick for CFW folders, loaders and plugin lines.
pub fn detect(paths: &PspPaths) -> CfwState {
    let mut state = CfwState::empty();

    state.ark4 = paths.savedata().join("ARK_01234").is_dir();
    state.ark5 = ["FasterARK", "ARK_05", "ARK_UPDATE"]
        .iter()
        .any(|name| has_dir(&paths.game(), name));
    state.pro = ["PROUPDATE", "635PROUPDATE", "FastRecovery"]
        .iter()
        .any(|name| has_dir(&paths.game(), name));
    state.me = has_dir(&paths.game(), "MEUPDATE");
    state.time_machine = paths.root.join("TM").is_dir();
    state.recovery = ["DC10", "ARK_DC"]
        .iter()
        .any(|name| has_dir(&paths.root, name) || has_dir(&paths.game(), name));
    state.iso_driver = dir_has_entries(&paths.iso());
    state.plugins = plugin_paths(&paths.seplugins());

    state
}

/// Ordered, plain-language plan for moving the stick to ARK-5 via FasterARK.
pub fn migration_steps(state: &CfwState) -> Vec<String> {
    let mut steps = Vec::new();

    if state.ark5 {
        steps.push(
            "ARK-5 is already on the stick. Boot the ARK Loader once and confirm it starts before removing any older loader.".to_string(),
        );
    } else {
        steps.push(
            "Back up PSP/SAVEDATA (saves) and the whole seplugins/ folder to your computer before changing anything.".to_string(),
        );
        steps.push(
            "Install FasterARK (ARK-5) from the install card. It copies the ARK_UPDATE PSP tree and the cIPL installer onto the stick.".to_string(),
        );
        steps.push(
            "Eject safely, then boot the ARK Loader on the console and confirm ARK-5 starts from the launcher.".to_string(),
        );
        steps.push(
            "Run the custom IPL installer inside ARK and choose New cIPL. This writes the firmware to flash and is permanent for the console, so only do it after ARK-5 boots reliably.".to_string(),
        );
    }

    let mut legacy: Vec<&str> = Vec::new();
    if state.ark4 {
        legacy.push("ARK-4 (PSP/SAVEDATA/ARK_01234 and its loader)");
    }
    if state.pro {
        legacy.push("PRO CFW (PROUPDATE / FastRecovery)");
    }
    if state.me {
        legacy.push("ME CFW (MEUPDATE)");
    }
    if legacy.is_empty() {
        steps.push(
            "Keep any older loader folders on the stick until one clean ARK-5 boot, then retire them one at a time.".to_string(),
        );
    } else {
        steps.push(format!(
            "After the first clean ARK-5 boot, retire the old loaders: {}. Remove only their loader folders, never the saves.",
            legacy.join(", ")
        ));
    }

    if state.recovery || state.time_machine {
        steps.push(
            "Keep the DC10 recovery folder (and TM/ if present). It is the unbrick path if a firmware write goes wrong.".to_string(),
        );
    } else {
        steps.push(
            "Keep a recovery path available. DC10 or Time Machine can be added before the cIPL write so a bad flash is recoverable.".to_string(),
        );
    }

    steps.push(
        "Re-check plugins in the Plugins tab after the upgrade: ARK-5 still reads the classic seplugins game.txt, vsh.txt and pops.txt lines.".to_string(),
    );

    steps
}

fn has_dir(parent: &Path, name: &str) -> bool {
    let direct = parent.join(name);
    if direct.is_dir() {
        return true;
    }
    // Memory Sticks are FAT, so match folder names case-insensitively too.
    let lowered = name.to_ascii_lowercase();
    match fs::read_dir(parent) {
        Ok(entries) => entries.flatten().any(|entry| {
            entry.file_type().map(|t| t.is_dir()).unwrap_or(false)
                && entry.file_name().to_string_lossy().to_ascii_lowercase() == lowered
        }),
        Err(_) => false,
    }
}

fn dir_has_entries(dir: &Path) -> bool {
    match fs::read_dir(dir) {
        Ok(mut entries) => entries.next().is_some(),
        Err(_) => false,
    }
}

fn plugin_paths(seplugins: &Path) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut files: Vec<PathBuf> = match fs::read_dir(seplugins) {
        Ok(entries) => entries
            .flatten()
            .filter(|entry| {
                entry.file_type().map(|t| t.is_file()).unwrap_or(false)
                    && entry
                        .path()
                        .extension()
                        .map(|ext| ext.eq_ignore_ascii_case("txt"))
                        .unwrap_or(false)
            })
            .map(|entry| entry.path())
            .collect(),
        Err(_) => Vec::new(),
    };
    files.sort();
    for file in files {
        let Ok(text) = fs::read_to_string(&file) else {
            continue;
        };
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let path = line.split_whitespace().next().unwrap_or("");
            if !path.is_empty() && !out.iter().any(|seen| seen == path) {
                out.push(path.to_string());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn scratch(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!("godsend-cfw-{name}-{}-{stamp}", std::process::id()))
    }

    #[test]
    fn detects_ark4_pro_and_plugins() {
        let root = scratch("synthetic");
        fs::create_dir_all(root.join("PSP/SAVEDATA/ARK_01234")).unwrap();
        fs::create_dir_all(root.join("PSP/GAME/PROUPDATE")).unwrap();
        fs::create_dir_all(root.join("seplugins")).unwrap();
        fs::write(
            root.join("seplugins/game.txt"),
            "ms0:/seplugins/cwcheat/cwcheat.prx 1\n",
        )
        .unwrap();
        fs::write(
            root.join("seplugins/vsh.txt"),
            "ms0:/seplugins/cxmb.prx 0\n",
        )
        .unwrap();

        let paths = PspPaths::new(&root);
        let state = detect(&paths);

        assert!(state.ark4);
        assert!(state.pro);
        assert!(!state.ark5);
        assert!(!state.me);
        assert!(!state.time_machine);
        assert!(!state.recovery);
        assert!(!state.iso_driver);
        assert_eq!(
            state.plugins,
            vec![
                "ms0:/seplugins/cwcheat/cwcheat.prx".to_string(),
                "ms0:/seplugins/cxmb.prx".to_string()
            ]
        );

        let steps = migration_steps(&state);
        assert!(steps.len() >= 5);
        assert!(steps.iter().any(|s| s.contains("FasterARK")));
        assert!(steps.iter().any(|s| s.contains("New cIPL")));
        assert!(steps.iter().any(|s| s.contains("ARK-4")));

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn empty_tree_reports_nothing() {
        let root = scratch("empty");
        fs::create_dir_all(root.join("PSP")).unwrap();

        let state = detect(&PspPaths::new(&root));

        assert!(!state.ark4);
        assert!(!state.ark5);
        assert!(!state.pro);
        assert!(!state.me);
        assert!(!state.time_machine);
        assert!(!state.recovery);
        assert!(!state.iso_driver);
        assert!(state.plugins.is_empty());

        let steps = migration_steps(&state);
        assert!(!steps.is_empty());
        assert!(steps[0].contains("Back up"));

        fs::remove_dir_all(&root).ok();
    }
}
