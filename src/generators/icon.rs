use image::ImageFormat;
use pelite::PeFile;
use std::path::Path;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kalesa_icon_test_{}_{}",
            label,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn invalid_pe_returns_false() {
        let dir = temp_dir("garbage");
        let output = dir.join("icon.png");
        assert!(!extract_pe_icon(b"not a pe file", &output));
        assert!(!output.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
