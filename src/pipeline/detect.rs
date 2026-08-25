use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use crate::domain::BinaryType;
use crate::error::{KalesaError, Result};

pub fn detect_binary_type(target_path: &Path) -> Result<BinaryType> {
    detect(target_path)
}

pub fn detect(target_path: &Path) -> Result<BinaryType> {
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
        if !confirm_pe_signature(&mut file)? {
            return Err(KalesaError::InvalidPe(target_path.to_path_buf()));
        }
        return Ok(BinaryType::WindowsPe);
    }

    Err(KalesaError::UnsupportedBinary(target_path.to_path_buf()))
}

fn confirm_pe_signature(file: &mut File) -> Result<bool> {
    file.seek(SeekFrom::Start(0x3C))
        .map_err(|e| KalesaError::io("seeking to PE offset", e))?;

    let mut offset_bytes = [0u8; 4];
    if file.read_exact(&mut offset_bytes).is_err() {
        return Ok(false);
    }

    let e_lfanew = u32::from_le_bytes(offset_bytes) as u64;
    if e_lfanew < 0x40 {
        return Ok(false);
    }

    file.seek(SeekFrom::Start(e_lfanew))
        .map_err(|e| KalesaError::io("seeking to PE signature", e))?;

    let mut pe_header = [0u8; 24];
    if file.read_exact(&mut pe_header).is_err() || &pe_header[..4] != b"PE\0\0" {
        return Ok(false);
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write as _;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEMP_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn write_temp(name: &str, bytes: &[u8]) -> PathBuf {
        let counter = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "kalesa_detect_{}_{}",
            std::process::id(),
            counter
        ));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(bytes).unwrap();
        path
    }

    #[test]
    fn detects_valid_elf() {
        let mut data = vec![0u8; 16];
        data[..4].copy_from_slice(b"\x7fELF");
        data[4] = 2;
        data[5] = 1;
        data[6] = 1;
        let path = write_temp("test.elf", &data);
        assert_eq!(detect(&path).unwrap(), BinaryType::LinuxElf);
    }

    #[test]
    fn detects_valid_pe_signature() {
        let mut data = vec![0u8; 0x40];
        data[0..2].copy_from_slice(b"MZ");
        data[0x3C..0x40].copy_from_slice(&(0x40u32).to_le_bytes());
        data.extend_from_slice(b"PE\0\0");
        data.extend_from_slice(&[0; 20]);
        let path = write_temp("test.exe", &data);
        assert_eq!(detect(&path).unwrap(), BinaryType::WindowsPe);
    }

    #[test]
    fn rejects_fake_mz() {
        let mut data = vec![0u8; 0x40];
        data[0..2].copy_from_slice(b"MZ");
        data[0x3C..0x40].copy_from_slice(&(0x40u32).to_le_bytes());
        data.extend_from_slice(b"BAD!");
        let path = write_temp("fake.exe", &data);
        assert!(matches!(detect(&path), Err(KalesaError::InvalidPe(_))));
    }
}
