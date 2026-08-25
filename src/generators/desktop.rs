use log::{info, warn};
use std::fs::File;
use std::io::Write;
use std::path::Path;

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
        Some(path) => desktop_value_escape(path.to_str().ok_or_else(|| {
            KalesaError::InvalidDesktopValue("icon path is not valid UTF-8".into())
        })?)?,
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
        "[Desktop Entry]\nType=Application\nName={name}\nExec={exec}\nIcon={icon}\nTerminal=false\nCategories={categories}\n"
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
    let mut file =
        File::create(path).map_err(|e| KalesaError::io("creating desktop entry", e))?;
    file.write_all(content.as_bytes())
        .map_err(|e| KalesaError::io("writing desktop entry", e))?;
    info!("Generated {:?}", path);
    Ok(())
}
