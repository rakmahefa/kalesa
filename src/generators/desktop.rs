use log::{info, warn};
use std::fs::File;
use std::io::Write;
use std::path::Path;

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

pub fn write(
    game_name: &str,
    current_dir: &Path,
    icon_path: Option<&Path>,
    output_dir: &Path,
    force: bool,
) -> Result<()> {
    let name = desktop_value_escape(game_name)?;
    let launcher_path = current_dir.join(".workdir/bin/launch.sh");
    let exec = desktop_exec_quote(&launcher_path)?;
    let icon = match icon_path {
        Some(path) => desktop_value_escape(path.to_str().ok_or_else(|| {
            KalesaError::InvalidDesktopValue("icon path is not valid UTF-8".into())
        })?)?,
        None => "applications-games".to_string(),
    };

    let desktop_content = format!(
        "[Desktop Entry]\nType=Application\nName={name}\nExec={exec}\nIcon={icon}\nTerminal=false\nCategories=Game;\n"
    );
    write_if_allowed(&output_dir.join("game.desktop"), &desktop_content, force)?;

    let directory_content =
        format!("[Desktop Entry]\nType=Directory\nName={name}\nIcon={icon}\n");
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
    use std::path::PathBuf;

    fn temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kalesa_desktop_test_{}_{}",
            label,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn escapes_names_and_paths() {
        let dir = temp_dir("quotes");
        let icon = dir.join("My Game's icon.png");
        write("My Game's Edition", &dir, Some(&icon), &dir, true).unwrap();
        let content = std::fs::read_to_string(dir.join("game.desktop")).unwrap();
        assert!(content.contains("Name=My Game's Edition"));
        assert!(content.contains("Exec=\""));
        assert!(content.contains("My Game's icon.png"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_newlines() {
        let dir = temp_dir("newline");
        let result = write("bad\nname", &dir, None, &dir, true);
        assert!(matches!(result, Err(KalesaError::InvalidDesktopValue(_))));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn respects_force_flag() {
        let dir = temp_dir("force");
        std::fs::write(dir.join("game.desktop"), "ORIGINAL").unwrap();
        write("My Game", &dir, None, &dir, false).unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("game.desktop")).unwrap(), "ORIGINAL");
        write("My Game", &dir, None, &dir, true).unwrap();
        assert!(std::fs::read_to_string(dir.join("game.desktop")).unwrap().contains("My Game"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
