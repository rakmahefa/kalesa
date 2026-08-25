pub mod config;
pub mod icon;
pub mod launcher;

use log::{info, warn};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// Detects if the binary is Windows (PE) or Linux (ELF).
///
/// For files starting with the "MZ" DOS-stub magic, this also checks for the
/// "PE\0\0" signature at the offset given by the DOS header's `e_lfanew`
/// field, rather than trusting "MZ" alone: any plain DOS executable also
/// starts with "MZ", so that check alone can't reliably distinguish a real
/// PE (.exe/.dll) from a non-PE DOS binary. If the PE signature can't be
/// confirmed, the file is still reported as Windows (matching the previous,
/// more lenient behavior) but a warning is logged so the mismatch is visible.
pub fn detect_binary_type(
    target_path: &Path,
) -> Result<(&'static str, bool), Box<dyn std::error::Error>> {
    let mut file = File::open(target_path)?;
    let mut magic = [0u8; 4];
    let read_bytes = file.read(&mut magic).unwrap_or(0);

    if read_bytes >= 4 && &magic == b"\x7fELF" {
        return Ok(("linux", false));
    }

    if read_bytes >= 2 && &magic[..2] == b"MZ" {
        match confirm_pe_signature(&mut file) {
            Ok(true) => return Ok(("windows", true)),
            Ok(false) => {
                warn!(
                    "{:?}: 'MZ' header found but no valid PE signature at e_lfanew; \
                     treating as Windows binary anyway",
                    target_path
                );
                return Ok(("windows", true));
            }
            Err(_) => {
                warn!(
                    "{:?}: could not verify PE signature (file too short?); \
                     treating as Windows binary anyway",
                    target_path
                );
                return Ok(("windows", true));
            }
        }
    }

    Ok(("unknown", false))
}

/// Reads the `e_lfanew` field from the DOS header (offset 0x3C) and checks
/// for the "PE\0\0" signature at that offset.
fn confirm_pe_signature(file: &mut File) -> std::io::Result<bool> {
    file.seek(SeekFrom::Start(0x3C))?;
    let mut offset_bytes = [0u8; 4];
    file.read_exact(&mut offset_bytes)?;
    let e_lfanew = u32::from_le_bytes(offset_bytes) as u64;

    file.seek(SeekFrom::Start(e_lfanew))?;
    let mut sig = [0u8; 4];
    file.read_exact(&mut sig)?;
    Ok(&sig == b"PE\0\0")
}

/// Initializes `.workdir` structure and generates configs and launch scripts.
///
/// `force` controls whether an already-existing `game.desktop` / `.directory`
/// is overwritten (see `launcher::write_desktop_entries`).
pub fn run_setup(
    target_path: &Path,
    custom_name: Option<String>,
    force: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if !target_path.exists() {
        return Err(format!("Target executable does not exist: {:?}", target_path).into());
    }

    let exe_filename = target_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("game.exe")
        .to_string();

    let game_name = custom_name.unwrap_or_else(|| {
        target_path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("Game")
            .to_string()
    });

    let current_dir = std::env::current_dir()?;
    let current_dir_str = current_dir.to_string_lossy().to_string();

    let (exe_type, is_windows) = detect_binary_type(target_path)?;
    info!("Detected target binary type: {}", exe_type);

    let workdir = Path::new(".workdir");
    let config_dir = workdir.join("config");
    let bin_dir = workdir.join("bin");
    let icons_dir = workdir.join("icons");

    fs::create_dir_all(&config_dir)?;
    fs::create_dir_all(&bin_dir)?;
    fs::create_dir_all(&icons_dir)?;

    let mut icon_extracted = false;
    let mut icon_extension = String::from("png");

    if is_windows {
        let icon_target_path = icons_dir.join("game_icon.png");
        if let Ok(mut target_file) = File::open(target_path) {
            let mut contents = Vec::new();
            if target_file.read_to_end(&mut contents).is_ok() {
                if icon::extract_pe_icon(&contents, &icon_target_path) {
                    icon_extracted = true;
                    info!("Extracted icon from PE resources to {:?}", icon_target_path);
                } else {
                    warn!(
                        "Could not extract an icon from the PE resources of {:?}",
                        target_path
                    );
                }
            }
        }
    } else {
        // Best-effort: PE resources don't apply to ELF binaries, so look for
        // a conventionally-named sibling icon file instead.
        if let Some(found) = icon::find_linux_icon(target_path) {
            let ext = found
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("png")
                .to_string();
            let icon_target_path = icons_dir.join(format!("game_icon.{}", ext));
            if fs::copy(&found, &icon_target_path).is_ok() {
                icon_extracted = true;
                icon_extension = ext;
                info!("Copied sibling icon {:?} to {:?}", found, icon_target_path);
            }
        } else {
            warn!("No sibling icon found next to {:?}", target_path);
        }
    }

    let desktop_icon_val = if icon_extracted {
        format!(
            "{}/.workdir/icons/game_icon.{}",
            current_dir_str, icon_extension
        )
    } else {
        "applications-games".to_string()
    };

    let wine_prefix = if is_windows {
        Some(format!("{}/.workdir/wine", current_dir_str))
    } else {
        None
    };

    let config_file_path = config_dir.join("config.yaml");
    config::write_config(
        &config_file_path,
        &game_name,
        &exe_filename,
        exe_type,
        is_windows,
        &current_dir_str,
    )?;
    info!("Generated {:?}", config_file_path);

    let launch_sh_path = bin_dir.join("launch.sh");
    launcher::write_launch_script(&launch_sh_path, &exe_filename, is_windows, wine_prefix.as_deref())?;
    info!("Generated {:?}", launch_sh_path);

    launcher::write_desktop_entries(
        &game_name,
        &current_dir_str,
        &desktop_icon_val,
        &current_dir,
        force,
    )?;

    info!("Architecture setup completed successfully for {}", game_name);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    fn write_temp(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("kalesa_bintest_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        let mut f = File::create(&path).unwrap();
        f.write_all(bytes).unwrap();
        path
    }

    #[test]
    fn detects_elf() {
        let path = write_temp("test.elf", b"\x7fELF\x02\x01\x01\x00");
        let (ty, is_win) = detect_binary_type(&path).unwrap();
        assert_eq!(ty, "linux");
        assert!(!is_win);
    }

    #[test]
    fn detects_real_pe_via_signature() {
        // Minimal DOS header: "MZ" magic + e_lfanew (offset 0x3C) pointing
        // right after the header, followed by the "PE\0\0" signature.
        let mut data = vec![0u8; 0x40];
        data[0] = b'M';
        data[1] = b'Z';
        data[0x3C..0x40].copy_from_slice(&(0x40u32).to_le_bytes());
        data.extend_from_slice(b"PE\0\0");

        let path = write_temp("test_real.exe", &data);
        let (ty, is_win) = detect_binary_type(&path).unwrap();
        assert_eq!(ty, "windows");
        assert!(is_win);
    }

    #[test]
    fn detects_mz_without_pe_signature_still_reports_windows() {
        // "MZ" header but garbage where the PE signature should be - old
        // behavior (treat as Windows) is preserved, just with a warning.
        let mut data = vec![0u8; 0x40];
        data[0] = b'M';
        data[1] = b'Z';
        data[0x3C..0x40].copy_from_slice(&(0x40u32).to_le_bytes());
        data.extend_from_slice(b"\x00\x00\x00\x00");

        let path = write_temp("test_fake.exe", &data);
        let (ty, is_win) = detect_binary_type(&path).unwrap();
        assert_eq!(ty, "windows");
        assert!(is_win);
    }

    #[test]
    fn detects_unknown() {
        let path = write_temp("test.bin", b"garbage data");
        let (ty, is_win) = detect_binary_type(&path).unwrap();
        assert_eq!(ty, "unknown");
        assert!(!is_win);
    }
}
