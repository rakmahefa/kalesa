use log::{info, warn};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::domain::GameMetadata;
use crate::error::{KalesaError, Result};

fn desktop_exec_quote(path: &Path) -> Result<String> {
    let value = path
        .to_str()
        .ok_or_else(|| KalesaError::InvalidDesktopValue("path is not valid UTF-8".into()))?;
    if value.chars().any(|ch| matches!(ch, '\n' | '\r')) {
        return Err(KalesaError::InvalidDesktopValue(
            "Exec path cannot contain a newline".into(),
        ));
    }
    let mut escaped = String::with_capacity(value.len() + 8);
    for ch in value.chars() {
        match ch {
            '\\' | '"' | '`' | '$' => {
                escaped.push('\\');
                escaped.push(ch);
            }
            _ => escaped.push(ch),
        }
    }
    Ok(format!("\"{escaped}\""))
}

fn desktop_value_escape(value: &str) -> Result<String> {
    if value.chars().any(|ch| matches!(ch, '\n' | '\r')) {
        return Err(KalesaError::InvalidDesktopValue(
            "Desktop Entry value cannot contain a newline".into(),
        ));
    }
    Ok(value.replace('\\', "\\\\"))
}

fn desktop_icon_path(path: &Path, current_dir: &Path) -> Result<PathBuf> {
    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else {
        current_dir.join(path)
    };

    if resolved.to_str().is_none() {
        return Err(KalesaError::InvalidDesktopValue(
            "icon path is not valid UTF-8".into(),
        ));
    }

    Ok(resolved)
}

pub fn write(
    game_name: &str,
    current_dir: &Path,
    icon_path: Option<&Path>,
    output_dir: &Path,
    force: bool,
) -> Result<()> {
    let metadata = GameMetadata {
        name: game_name.to_string(),
        icon_path: icon_path.map(Path::to_path_buf),
        ..GameMetadata::default()
    };
    write_with_metadata(&metadata, current_dir, output_dir, force)
}

pub fn write_with_metadata(
    metadata: &GameMetadata,
    current_dir: &Path,
    output_dir: &Path,
    force: bool,
) -> Result<()> {
    let name = desktop_value_escape(&metadata.name)?;
    let launcher_path = current_dir.join(".workdir/bin/launch.sh");
    let exec = desktop_exec_quote(&launcher_path)?;
    let icon = match metadata.icon_path.as_deref() {
        Some(path) => {
            let path = desktop_icon_path(path, current_dir)?;
            desktop_value_escape(path.to_str().ok_or_else(|| {
                KalesaError::InvalidDesktopValue("icon path is not valid UTF-8".into())
            })?)?
        }
        None => "applications-games".to_string(),
    };

    let version = metadata
        .version
        .as_deref()
        .map(desktop_value_escape)
        .transpose()?;
    let developer = metadata
        .developer
        .as_deref()
        .map(desktop_value_escape)
        .transpose()?;
    let comment = metadata
        .description
        .as_deref()
        .map(desktop_value_escape)
        .transpose()?;
    let categories = if metadata.categories.is_empty() {
        "Game;".to_string()
    } else {
        format!(
            "{};",
            metadata
                .categories
                .iter()
                .map(|v| desktop_value_escape(v))
                .collect::<Result<Vec<_>>>()?
                .join(";")
        )
    };

    let mut desktop_content = format!(
        "[Desktop Entry]\nType=Application\nName={name}\nExec={exec}\nIcon={icon}\nTerminal=true\nCategories={categories}\n"
    );
    if let Some(value) = version {
        desktop_content.push_str(&format!("X-Kalesa-Version={value}\n"));
    }
    if let Some(value) = developer {
        desktop_content.push_str(&format!("X-Kalesa-Developer={value}\n"));
    }
    if let Some(value) = comment {
        desktop_content.push_str(&format!("Comment={value}\n"));
    }

    write_if_allowed(&output_dir.join("game.desktop"), &desktop_content, force)?;

    let directory_content = format!("[Desktop Entry]\nType=Directory\nName={name}\nIcon={icon}\n");
    write_if_allowed(&output_dir.join(".directory"), &directory_content, force)?;
    Ok(())
}

fn write_if_allowed(path: &Path, content: &str, force: bool) -> Result<()> {
    if path.exists() && !force {
        warn!(
            "{:?} already exists, skipping (pass --force to overwrite)",
            path
        );
        return Ok(());
    }
    let mut file = File::create(path).map_err(|e| KalesaError::io("creating desktop entry", e))?;
    file.write_all(content.as_bytes())
        .map_err(|e| KalesaError::io("writing desktop entry", e))?;
    info!("Generated {:?}", path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kalesa_desktop_test_{label}_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn resolves_relative_icon_path_to_absolute_path() {
        let current_dir = PathBuf::from("/tmp/kalesa-test");
        let relative = Path::new(".workdir/icons/game_icon.png");
        let resolved = desktop_icon_path(relative, &current_dir).unwrap();
        assert_eq!(
            resolved,
            PathBuf::from("/tmp/kalesa-test/.workdir/icons/game_icon.png")
        );
    }

    #[test]
    fn preserves_absolute_icon_path() {
        let current_dir = PathBuf::from("/tmp/kalesa-test");
        let absolute = Path::new("/opt/games/icons/game_icon.png");
        let resolved = desktop_icon_path(absolute, &current_dir).unwrap();
        assert_eq!(resolved, absolute);
    }

    #[test]
    fn writes_absolute_icon_path_to_desktop_entries() {
        let dir = temp_dir("absolute_icon");
        let icon_path = Path::new(".workdir/icons/game_icon.png");
        let metadata = GameMetadata {
            name: "ChildofLight".to_string(),
            icon_path: Some(icon_path.to_path_buf()),
            ..GameMetadata::default()
        };

        write_with_metadata(&metadata, &dir, &dir, true).unwrap();

        let desktop = fs::read_to_string(dir.join("game.desktop")).unwrap();
        let directory = fs::read_to_string(dir.join(".directory")).unwrap();
        let expected = format!("Icon={}/.workdir/icons/game_icon.png", dir.display());

        assert!(desktop.contains(&expected), "desktop content: {desktop}");
        assert!(
            directory.contains(&expected),
            "directory content: {directory}"
        );

        let _ = fs::remove_dir_all(&dir);
    }
}
