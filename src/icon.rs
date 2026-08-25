use image::ImageFormat;
use log::debug;
use pelite::PeFile;
use std::path::{Path, PathBuf};

/// Extracts the best available icon from a Windows PE file's resources and
/// saves it as a PNG at `output_path`. Returns `true` on success.
pub fn extract_pe_icon(pe_data: &[u8], output_path: &Path) -> bool {
    let pe = match PeFile::from_bytes(pe_data) {
        Ok(file) => file,
        Err(_) => return false,
    };

    let resources = match pe.resources() {
        Ok(res) => res,
        Err(_) => return false,
    };

    for group_res in resources.icons() {
        let (_name, group) = match group_res {
            Ok(g) => g,
            Err(_) => continue,
        };

        // NOTE: we cannot call image::load_from_memory() directly on the raw
        // bytes of a single entry (group.image(id)). For every size that
        // isn't PNG-encoded (typically everything except the 256x256), the
        // resource is a "headerless" DIB (BITMAPINFOHEADER + XOR bits + AND
        // mask) WITHOUT the BITMAPFILEHEADER ("BM"...) that a standalone BMP
        // decoder requires. image::load_from_memory can't recognize the
        // format and fails.
        //
        // The `image` crate *can* decode this DIB, but only through its
        // internal ICO decoder (BmpDecoder::new_with_ico_format), which is
        // only reachable by handing it a complete, valid .ico file.
        //
        // pelite provides group.write() precisely to reassemble a full .ico
        // (ICONDIR header + entries + data), which we can then decode with
        // ImageFormat::Ico. This also fixes "best size" selection: the ICO
        // decoder's internal selector correctly treats width/height == 0 as
        // meaning 256 (the old code compared bWidth*bHeight as u8, so a
        // 256x256 icon scored 0x0=0 and was always passed over in favor of a
        // smaller, non-PNG icon that then failed to decode anyway).
        let mut ico_bytes = Vec::new();
        if group.write(&mut ico_bytes).is_err() {
            continue;
        }

        if let Ok(img) = image::load_from_memory_with_format(&ico_bytes, ImageFormat::Ico) {
            if img.save_with_format(output_path, ImageFormat::Png).is_ok() {
                return true;
            }
        }
    }

    false
}

/// Best-effort icon lookup for Linux/ELF targets.
///
/// PE resources don't apply to ELF binaries, so instead of leaving Linux
/// games without any extracted icon, look for a conventionally-named icon
/// file next to the executable (a common pattern for Linux game
/// distributions). Returns the path to the first match found, if any.
pub fn find_linux_icon(target_path: &Path) -> Option<PathBuf> {
    let dir = target_path.parent()?;
    let stem = target_path.file_stem()?.to_str()?;

    let candidates = [
        "icon.png".to_string(),
        "icon.svg".to_string(),
        "icon.xpm".to_string(),
        format!("{}.png", stem),
        format!("{}.svg", stem),
    ];

    for candidate in &candidates {
        let path = dir.join(candidate);
        if path.is_file() {
            debug!("Found sibling icon candidate: {:?}", path);
            return Some(path);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kalesa_icon_test_{}_{}",
            label,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn extract_pe_icon_returns_false_on_garbage_input() {
        let dir = temp_dir("garbage");
        let out = dir.join("icon.png");

        assert!(!extract_pe_icon(b"not a pe file", &out));
        assert!(!out.exists());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn find_linux_icon_finds_sibling_png() {
        let dir = temp_dir("sibling_png");
        let target = dir.join("mygame");
        fs::write(&target, b"fake elf").unwrap();
        let icon_path = dir.join("icon.png");
        fs::write(&icon_path, b"fake png").unwrap();

        assert_eq!(find_linux_icon(&target), Some(icon_path));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn find_linux_icon_finds_name_based_svg() {
        let dir = temp_dir("name_svg");
        let target = dir.join("mygame");
        fs::write(&target, b"fake elf").unwrap();
        let icon_path = dir.join("mygame.svg");
        fs::write(&icon_path, b"fake svg").unwrap();

        assert_eq!(find_linux_icon(&target), Some(icon_path));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn find_linux_icon_returns_none_when_absent() {
        let dir = temp_dir("none");
        let target = dir.join("mygame");
        fs::write(&target, b"fake elf").unwrap();

        assert_eq!(find_linux_icon(&target), None);

        let _ = fs::remove_dir_all(&dir);
    }
}
