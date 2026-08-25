use std::path::{Path, PathBuf};

use crate::domain::{BinaryType, GameMetadata, GameTarget};

pub fn collect(target: &GameTarget, custom_name: Option<&str>) -> GameMetadata {
    let name = custom_name
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| default_name(&target.path));

    let icon_path = (target.binary_type == BinaryType::LinuxElf)
        .then(|| find_linux_icon(&target.path))
        .flatten();

    GameMetadata::new(name, icon_path)
}

fn default_name(target_path: &Path) -> String {
    target_path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("Game")
        .to_string()
}

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

    candidates
        .iter()
        .map(|candidate| dir.join(candidate))
        .find(|path| path.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kalesa_metadata_test_{}_{}",
            label,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn derives_default_name() {
        let dir = temp_dir("name");
        let target_path = dir.join("mygame");
        let target = GameTarget::new(target_path, BinaryType::LinuxElf);
        let metadata = collect(&target, None);
        assert_eq!(metadata.name, "mygame");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn finds_sibling_icon() {
        let dir = temp_dir("icon");
        let target_path = dir.join("mygame");
        fs::write(&target_path, b"fake").unwrap();
        let icon = dir.join("icon.png");
        fs::write(&icon, b"fake png").unwrap();
        let target = GameTarget::new(target_path, BinaryType::LinuxElf);
        assert_eq!(find_linux_icon(&target.path), Some(icon));
        let _ = fs::remove_dir_all(&dir);
    }
}
