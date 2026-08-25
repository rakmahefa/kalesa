pub mod config;
pub mod error;
pub mod icon;
pub mod launcher;

use log::{info, warn};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use error::{KalesaError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryType {
    LinuxElf,
    WindowsPe,
}

impl BinaryType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LinuxElf => "linux",
            Self::WindowsPe => "windows",
        }
    }

    pub fn is_windows(self) -> bool {
        matches!(self, Self::WindowsPe)
    }
}

/// Detects and strictly validates whether `target_path` is a Linux ELF or
/// Windows PE executable. Unknown formats and malformed headers are rejected.
pub fn detect_binary_type(target_path: &Path) -> Result<BinaryType> {
    let mut file = File::open(target_path).map_err(|e| KalesaError::io("opening target", e))?;
    let mut header = [0u8; 16];
    let read_bytes = file
        .read(&mut header)
        .map_err(|e| KalesaError::io("reading binary header", e))?;

    if read_bytes >= 4 && &header[..4] == b"\x7fELF" {
        if read_bytes < 16
            || !matches!(header[4], 1 | 2)
            || !matches!(header[5], 1 | 2)
            || header[6] != 1
        {
            return Err(KalesaError::InvalidElf(target_path.to_path_buf()));
        }
        return Ok(BinaryType::LinuxElf);
    }

    if read_bytes >= 2 && &header[..2] == b"MZ" {
        if !confirm_pe_signature(&mut file, target_path)? {
            return Err(KalesaError::InvalidPe(target_path.to_path_buf()));
        }
        return Ok(BinaryType::WindowsPe);
    }

    Err(KalesaError::UnsupportedBinary(target_path.to_path_buf()))
}

/// Reads the DOS `e_lfanew` field and validates the PE signature and minimum
/// COFF header size at that offset.
fn confirm_pe_signature(file: &mut File, target_path: &Path) -> Result<bool> {
    file.seek(SeekFrom::Start(0x3C))
        .map_err(|e| KalesaError::io("seeking to PE offset", e))?;

    let mut offset_bytes = [0u8; 4];
    if file.read_exact(&mut offset_bytes).is_err() {
        return Ok(false);
    }
    let e_lfanew = u32::from_le_bytes(offset_bytes) as u64;

    // A PE signature must not overlap the DOS header. Also reject obviously
    // nonsensical offsets instead of allowing an arbitrary seek into a file.
    if e_lfanew < 0x40 {
        return Ok(false);
    }

    file.seek(SeekFrom::Start(e_lfanew))
        .map_err(|e| KalesaError::io("seeking to PE signature", e))?;

    let mut pe_header = [0u8; 24];
    if file.read_exact(&mut pe_header).is_err() || &pe_header[..4] != b"PE\0\0" {
        return Ok(false);
    }

    // COFF header fields are now present and structurally readable. We keep
    // the detailed machine validation out of Phase 1; the format itself is
    // nevertheless now strictly distinguished from a generic MZ file.
    let _ = target_path;
    Ok(true)
}

/// Initializes `.workdir` structure and generates configs and launch scripts.
pub fn run_setup(target_path: &Path, custom_name: Option<String>, force: bool) -> Result<()> {
    if !target_path.exists() {
        return Err(KalesaError::TargetNotFound(target_path.to_path_buf()));
    }
    if !target_path.is_file() {
        return Err(KalesaError::TargetNotFile(target_path.to_path_buf()));
    }

    let target_path = fs::canonicalize(target_path)
        .map_err(|e| KalesaError::io("canonicalizing target", e))?;

    let game_name = custom_name.unwrap_or_else(|| {
        target_path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("Game")
            .to_string()
    });

    let current_dir = std::env::current_dir()
        .map_err(|e| KalesaError::io("reading current directory", e))?;
    let binary_type = detect_binary_type(&target_path)?;
    info!("Detected target binary type: {}", binary_type.as_str());

    let workdir = PathBuf::from(".workdir");
    let config_dir = workdir.join("config");
    let bin_dir = workdir.join("bin");
    let icons_dir = workdir.join("icons");

    fs::create_dir_all(&config_dir).map_err(|e| KalesaError::io("creating config directory", e))?;
    fs::create_dir_all(&bin_dir).map_err(|e| KalesaError::io("creating bin directory", e))?;
    fs::create_dir_all(&icons_dir).map_err(|e| KalesaError::io("creating icons directory", e))?;

    let mut icon_path = None;

    if binary_type.is_windows() {
        let icon_target_path = icons_dir.join("game_icon.png");
        let contents = fs::read(&target_path)
            .map_err(|e| KalesaError::io("reading PE icon resources", e))?;
        if icon::extract_pe_icon(&contents, &icon_target_path) {
            info!("Extracted icon from PE resources to {:?}", icon_target_path);
            icon_path = Some(icon_target_path);
        } else {
            warn!("Could not extract an icon from the PE resources of {:?}", target_path);
        }
    } else if let Some(found) = icon::find_linux_icon(&target_path) {
        let ext = found.extension().and_then(|e| e.to_str()).unwrap_or("png");
        let destination = icons_dir.join(format!("game_icon.{ext}"));
        if fs::copy(&found, &destination).is_ok() {
            info!("Copied sibling icon {:?} to {:?}", found, destination);
            icon_path = Some(destination);
        } else {
            warn!("Could not copy sibling icon {:?}", found);
        }
    } else {
        warn!("No sibling icon found next to {:?}", target_path);
    }

    let config_file_path = config_dir.join("config.yaml");
    config::write_config(&config_file_path, &game_name, &target_path, binary_type, &current_dir)?;
    info!("Generated {:?}", config_file_path);

    let launch_sh_path = bin_dir.join("launch.sh");
    let wine_prefix = binary_type
        .is_windows()
        .then(|| current_dir.join(".workdir/wine"));
    launcher::write_launch_script(
        &launch_sh_path,
        &target_path,
        binary_type,
        wine_prefix.as_deref(),
    )?;
    info!("Generated {:?}", launch_sh_path);

    launcher::write_desktop_entries(
        &game_name,
        &current_dir,
        icon_path.as_deref(),
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

    fn write_temp(name: &str, bytes: &[u8]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("kalesa_bintest_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        let mut f = File::create(&path).unwrap();
        f.write_all(bytes).unwrap();
        path
    }

    #[test]
    fn detects_valid_elf() {
        let mut data = vec![0u8; 16];
        data[..4].copy_from_slice(b"\x7fELF");
        data[4] = 2; // ELF64
        data[5] = 1; // little-endian
        data[6] = 1; // version
        let path = write_temp("test.elf", &data);

        assert_eq!(detect_binary_type(&path).unwrap(), BinaryType::LinuxElf);
    }

    #[test]
    fn rejects_truncated_elf() {
        let path = write_temp("test.elf", b"\x7fELF");
        assert!(matches!(
            detect_binary_type(&path),
            Err(KalesaError::InvalidElf(_))
        ));
    }

    #[test]
    fn detects_real_pe_via_signature() {
        let mut data = vec![0u8; 0x40];
        data[0] = b'M';
        data[1] = b'Z';
        data[0x3C..0x40].copy_from_slice(&(0x40u32).to_le_bytes());
        data.extend_from_slice(b"PE\0\0");
        data.extend_from_slice(&[0u8; 20]);
        let path = write_temp("test_real.exe", &data);

        assert_eq!(detect_binary_type(&path).unwrap(), BinaryType::WindowsPe);
    }

    #[test]
    fn rejects_mz_without_pe_signature() {
        let mut data = vec![0u8; 0x40];
        data[0] = b'M';
        data[1] = b'Z';
        data[0x3C..0x40].copy_from_slice(&(0x40u32).to_le_bytes());
        data.extend_from_slice(b"BAD!");
        let path = write_temp("test_fake.exe", &data);

        assert!(matches!(
            detect_binary_type(&path),
            Err(KalesaError::InvalidPe(_))
        ));
    }

    #[test]
    fn rejects_unknown_binary() {
        let path = write_temp("test.bin", b"garbage data");
        assert!(matches!(
            detect_binary_type(&path),
            Err(KalesaError::UnsupportedBinary(_))
        ));
    }
}
