use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::{BinaryType, GameMetadata, GameTarget};

pub fn collect(target: &GameTarget, custom_name: Option<&str>) -> GameMetadata {
    let mut metadata = GameMetadata::new(
        custom_name
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| default_name(&target.path)),
        None,
    );

    if target.binary_type.is_linux() {
        if let Some(desktop) = find_desktop_file(&target.path) {
            let parsed = parse_desktop_metadata(&desktop);
            if custom_name.is_none() {
                if let Some(name) = parsed.name {
                    metadata.name = name;
                }
            }
            metadata.icon_path = parsed.icon.as_deref().and_then(|icon| {
                find_xdg_icon(&target.path, icon).or_else(|| find_sibling_icon(&target.path))
            });
            metadata.version = parsed.version;
            metadata.developer = parsed.developer;
            metadata.description = parsed.description;
            metadata.categories = parsed.categories;
        }

        if metadata.icon_path.is_none() {
            metadata.icon_path = find_sibling_icon(&target.path).or_else(|| {
                find_xdg_icon(&target.path, &metadata.name.to_ascii_lowercase())
            });
        }
    }

    metadata
}

fn default_name(target_path: &Path) -> String {
    target_path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("Game")
        .trim_end_matches('.')
        .to_string()
}

pub fn find_linux_icon(target_path: &Path) -> Option<PathBuf> {
    find_sibling_icon(target_path).or_else(|| {
        target_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .and_then(|stem| find_xdg_icon(target_path, stem))
    })
}

fn find_sibling_icon(target_path: &Path) -> Option<PathBuf> {
    let dir = target_path.parent()?;
    let stem = target_path.file_stem()?.to_str()?;
    let candidates = [
        "icon.png".to_string(),
        "icon.svg".to_string(),
        "icon.xpm".to_string(),
        format!("{}.png", stem),
        format!("{}.svg", stem),
        format!("{}.xpm", stem),
        ".DirIcon".to_string(),
    ];
    candidates
        .iter()
        .map(|candidate| dir.join(candidate))
        .find(|path| path.is_file())
}

fn find_desktop_file(target_path: &Path) -> Option<PathBuf> {
    let dir = target_path.parent()?;
    let stem = target_path.file_stem()?.to_str()?;
    let direct = dir.join(format!("{stem}.desktop"));
    if direct.is_file() {
        return Some(direct);
    }
    fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.extension().and_then(|e| e.to_str()) == Some("desktop"))
}

#[derive(Default)]
struct ParsedDesktop {
    name: Option<String>,
    icon: Option<String>,
    version: Option<String>,
    developer: Option<String>,
    description: Option<String>,
    categories: Vec<String>,
}

fn parse_desktop_metadata(path: &Path) -> ParsedDesktop {
    let mut parsed = ParsedDesktop::default();
    let Ok(content) = fs::read_to_string(path) else {
        return parsed;
    };
    for line in content.lines() {
        let Some((key, value)) = line.split_once('=') else { continue };
        let value = value.trim();
        match key {
            "Name" if parsed.name.is_none() => parsed.name = Some(value.to_string()),
            "Icon" if parsed.icon.is_none() => parsed.icon = Some(value.to_string()),
            "Version" | "X-AppImage-Version" if parsed.version.is_none() => {
                parsed.version = Some(value.to_string())
            }
            "X-AppImage-Name" if parsed.name.is_none() => parsed.name = Some(value.to_string()),
            "Developer" | "X-Developer" if parsed.developer.is_none() => {
                parsed.developer = Some(value.to_string())
            }
            "Comment" if parsed.description.is_none() => {
                parsed.description = Some(value.to_string())
            }
            "Categories" => {
                parsed.categories = value
                    .split(';')
                    .filter(|s| !s.is_empty())
                    .map(ToOwned::to_owned)
                    .collect();
            }
            _ => {}
        }
    }
    parsed
}

fn find_xdg_icon(target_path: &Path, name: &str) -> Option<PathBuf> {
    if name.is_empty() || name.contains('/') {
        let path = PathBuf::from(name);
        if path.is_absolute() && path.is_file() {
            return Some(path);
        }
    }

    let mut roots = Vec::new();
    if let Ok(home) = env::var("XDG_DATA_HOME") {
        if !home.is_empty() {
            roots.push(PathBuf::from(home));
        }
    } else if let Ok(home) = env::var("HOME") {
        roots.push(PathBuf::from(home).join(".local/share"));
    }

    let data_dirs = env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    roots.extend(data_dirs.split(':').filter(|s| !s.is_empty()).map(PathBuf::from));

    let icon_exts = ["png", "svg", "xpm"];
    let sizes = ["scalable", "512x512", "256x256", "128x128", "64x64", "48x48", "32x32", "24x24", "16x16"];

    for root in &roots {
        for size in sizes {
            for ext in icon_exts {
                let candidate = root
                    .join("icons/hicolor")
                    .join(size)
                    .join("apps")
                    .join(format!("{name}.{ext}"));
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
        for ext in icon_exts {
            let candidate = root.join("pixmaps").join(format!("{name}.{ext}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    target_path.parent().map(|dir| dir.join(name)).filter(|p| p.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("kalesa_metadata_test_{label}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn derives_default_name() {
        let dir = temp_dir("name");
        let target = GameTarget::new(dir.join("mygame"), BinaryType::LinuxElf);
        assert_eq!(collect(&target, None).name, "mygame");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn reads_desktop_metadata() {
        let dir = temp_dir("desktop");
        fs::write(dir.join("mygame"), b"fake").unwrap();
        fs::write(
            dir.join("mygame.desktop"),
            "[Desktop Entry]\nName=My Game\nIcon=mygame\nX-AppImage-Version=2.1\nComment=Great game\nCategories=Game;Emulator;\n",
        )
        .unwrap();
        let target = GameTarget::new(dir.join("mygame"), BinaryType::LinuxElf);
        let metadata = collect(&target, None);
        assert_eq!(metadata.name, "My Game");
        assert_eq!(metadata.version.as_deref(), Some("2.1"));
        assert_eq!(metadata.description.as_deref(), Some("Great game"));
        assert_eq!(metadata.categories, vec!["Game", "Emulator"]);
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
