use crate::config::UmiConfig;
use anyhow::{Context, Result};
use directories::BaseDirs;
use std::fs;
use std::path::PathBuf;

const UMI_ICON_SVG: &str = include_str!("../../assets/icons/umi-logo.svg");

fn is_running_in_appimage() -> bool {
    std::env::var_os("APPIMAGE").is_some()
}

fn current_exec_path() -> String {
    if let Ok(appimage) = std::env::var("APPIMAGE") {
        if !appimage.trim().is_empty() {
            return appimage;
        }
    }
    std::env::current_exe()
        .ok()
        .and_then(|p| p.to_str().map(str::to_string))
        .unwrap_or_else(|| "umi".to_string())
}

fn desktop_entry_contents(exec: &str, autostart: bool) -> String {
    let mut s = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=UMI\n\
         GenericName=Ambient Notch Companion\n\
         Comment=Dynamic Island desktop overlay for KDE Plasma Wayland\n\
         Exec={exec}\n\
         Icon=org.umi.Notch\n\
         Terminal=false\n\
         Categories=Utility;DesktopUtility;\n\
         StartupNotify=false\n\
         StartupWMClass=org.umi.Notch\n"
    );
    if autostart {
        s.push_str("X-GNOME-Autostart-enabled=true\n");
        s.push_str("X-KDE-autostart-after=panel\n");
    }
    s
}

/// Install `~/.local/share/applications/org.umi.Notch.desktop` and hicolor SVG icon
/// so KDE Plasma portals (GlobalShortcuts, Screenshot) and Kickoff recognize UMI by name & icon.
/// Skipped when running inside an AppImage so managers like Gear Lever own the launcher cleanly.
pub fn install_desktop_entry() -> Result<()> {
    if is_running_in_appimage() {
        return Ok(());
    }
    let exec = current_exec_path();

    if let Some(base) = BaseDirs::new() {
        let data_dir = base.data_local_dir();
        let icon_dir = data_dir.join("icons/hicolor/scalable/apps");
        fs::create_dir_all(&icon_dir)?;
        let icon_path = icon_dir.join("org.umi.Notch.svg");
        let _ = fs::write(&icon_path, UMI_ICON_SVG);

        let app_dir = data_dir.join("applications");
        fs::create_dir_all(&app_dir)?;
        let desktop_path = app_dir.join("org.umi.Notch.desktop");
        fs::write(&desktop_path, desktop_entry_contents(&exec, false))
            .with_context(|| format!("writing {}", desktop_path.display()))?;
    }
    Ok(())
}

/// Enable or disable `~/.config/autostart/umi.desktop` for KDE Plasma login autostart.
pub fn sync_autostart(enabled: bool) -> Result<()> {
    let base = BaseDirs::new().context("XDG base directories unavailable")?;
    let autostart_dir = base.config_dir().join("autostart");
    let path = autostart_dir.join("umi.desktop");

    if enabled {
        fs::create_dir_all(&autostart_dir)?;
        let exec = current_exec_path();
        fs::write(&path, desktop_entry_contents(&exec, true))
            .with_context(|| format!("writing {}", path.display()))?;
        tracing::info!("Autostart enabled at {}", path.display());
    } else if path.exists() {
        fs::remove_file(&path).with_context(|| format!("removing {}", path.display()))?;
        tracing::info!("Autostart disabled (removed {})", path.display());
    }
    Ok(())
}

/// Export current `UmiConfig` as pretty JSON to `~/.config/umi/umi-settings-export.json`.
pub fn export_settings_json(cfg: &UmiConfig) -> Result<PathBuf> {
    let base = BaseDirs::new().context("XDG base directories unavailable")?;
    let dir = base.config_dir().join("umi");
    fs::create_dir_all(&dir)?;
    let path = dir.join("umi-settings-export.json");
    let json = serde_json::to_string_pretty(cfg)?;
    fs::write(&path, json)?;
    Ok(path)
}
