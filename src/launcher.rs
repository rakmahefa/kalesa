use log::{info, warn};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use crate::error::{KalesaError, Result};
use crate::BinaryType;

/// Quotes an arbitrary shell word using POSIX single-quote rules.
fn shell_quote(value: &Path) -> Result<String> {
    let value = value
        .to_str()
        .ok_or_else(|| KalesaError::InvalidDesktopValue("path is not valid UTF-8".into()))?;
    Ok(format!("'{}'", value.replace('\'', "'\\''")))
}

/// Quotes one path argument for the Desktop Entry `Exec=` field.
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

/// Escapes a normal Desktop Entry string value.
fn desktop_value_escape(value: &str) -> Result<String> {
    if value.chars().any(|ch| matches!(ch, '\n' | '\r')) {
        return Err(KalesaError::InvalidDesktopValue(
            "Desktop Entry value cannot contain a newline".into(),
        ));
    }
    Ok(value.replace('\\', "\\\\"))
}

/// Generates `.workdir/bin/launch.sh`.
pub fn write_launch_script(
    path: &Path,
    executable_path: &Path,
    binary_type: BinaryType,
    wine_prefix: Option<&Path>,
) -> Result<()> {
    let executable = shell_quote(executable_path)?;

    let run_block = match binary_type {
        BinaryType::WindowsPe => {
            let prefix = wine_prefix.ok_or(KalesaError::MissingWinePrefix)?;
            let prefix = shell_quote(prefix)?;
            format!(
                "if ! command -v wine >/dev/null 2>&1; then\n    echo \"[!] 'wine' not found in PATH. Install wine to launch this Windows game.\" >&2\n    exit 1\nfi\n\nexport WINEPREFIX={prefix}\nexport WINEARCH=win64\n\necho \"[+] Launching Windows game via Wine...\"\nexec wine {executable} \"$@\"\n"
            )
        }
        BinaryType::LinuxElf => format!(
            "TARGET={executable}\nif [ ! -x \"$TARGET\" ]; then\n    chmod +x \"$TARGET\" 2>/dev/null || true\nfi\n\necho \"[+] Launching Linux game...\"\nexec \"$TARGET\" \"$@\"\n"
        ),
    };

    let content = format!(
        "#!/bin/bash\n# Generated launch script - do not edit by hand, re-run kalesa instead.\nset -euo pipefail\n\nSCRIPT_DIR=\"$(cd \"$(dirname \"${{BASH_SOURCE[0]}}\")\" && pwd)\"\nBASE_DIR=\"$(dirname \"$SCRIPT_DIR\")\"\nGAME_DIR=\"$(dirname \"$BASE_DIR\")\"\n\ncd \"$GAME_DIR\"\n\nif [ ! -f \".workdir/config/config.yaml\" ]; then\n    echo \"[!] .workdir/config/config.yaml not found - setup may be incomplete.\" >&2\n    exit 1\nfi\n\n{run_block}"
    );

    let mut file = File::create(path).map_err(|e| KalesaError::io("creating launch script", e))?;
    file.write_all(content.as_bytes())
        .map_err(|e| KalesaError::io("writing launch script", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755))
            .map_err(|e| KalesaError::io("setting launch script permissions", e))?;
    }

    Ok(())
}

/// Writes `game.desktop` and `.directory` inside `output_dir`.
pub fn write_desktop_entries(
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
    use std::fs;
    use std::path::PathBuf;

    fn temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kalesa_launcher_test_{}_{}",
            label,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn shell_quote_handles_spaces_and_single_quotes() {
        let path = Path::new("/tmp/My Game's [1998].exe");
        assert_eq!(
            shell_quote(path).unwrap(),
            r#"'/tmp/My Game'\''s [1998].exe'"#
        );
    }

    #[test]
    fn launch_script_windows_uses_safe_quoting() {
        let dir = temp_dir("win");
        let path = dir.join("launch.sh");
        let target = Path::new("/tmp/My Game's.exe");
        let prefix = Path::new("/tmp/Wine Prefix's");

        write_launch_script(&path, target, BinaryType::WindowsPe, Some(prefix)).unwrap();
        let content = fs::read_to_string(&path).unwrap();

        assert!(content.contains("exec wine '/tmp/My Game'\\''s.exe'"));
        assert!(content.contains("WINEPREFIX='/tmp/Wine Prefix'\\''s'"));
        assert!(content.starts_with("#!/bin/bash"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn launch_script_linux_runs_absolute_target_safely() {
        let dir = temp_dir("linux");
        let path = dir.join("launch.sh");
        let target = Path::new("/tmp/My Game's");

        write_launch_script(&path, target, BinaryType::LinuxElf, None).unwrap();
        let content = fs::read_to_string(&path).unwrap();

        assert!(content.contains("TARGET='/tmp/My Game'\\''s'"));
        assert!(!content.contains("wine"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn desktop_entries_escape_paths_and_names() {
        let dir = temp_dir("desktop_quotes");
        let icon = dir.join("My Game's icon.png");

        write_desktop_entries("My Game's Edition", &dir, Some(&icon), &dir, true).unwrap();

        let content = fs::read_to_string(dir.join("game.desktop")).unwrap();
        assert!(content.contains("Name=My Game's Edition"));
        assert!(content.contains("Exec=\""));
        assert!(content.contains("Icon=/tmp") || content.contains("Icon="));
        assert!(content.contains("My Game's icon.png"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn desktop_entries_reject_newlines() {
        let dir = temp_dir("desktop_newline");
        let result = write_desktop_entries("bad\nname", &dir, None, &dir, true);
        assert!(matches!(result, Err(KalesaError::InvalidDesktopValue(_))));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn desktop_entries_are_not_overwritten_without_force() {
        let dir = temp_dir("desktop_noforce");
        fs::write(dir.join("game.desktop"), "ORIGINAL").unwrap();

        write_desktop_entries("My Game", &dir, None, &dir, false).unwrap();

        let content = fs::read_to_string(dir.join("game.desktop")).unwrap();
        assert_eq!(content, "ORIGINAL");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn desktop_entries_are_overwritten_with_force() {
        let dir = temp_dir("desktop_force");
        fs::write(dir.join("game.desktop"), "ORIGINAL").unwrap();

        write_desktop_entries("My Game", &dir, None, &dir, true).unwrap();

        let content = fs::read_to_string(dir.join("game.desktop")).unwrap();
        assert!(content.contains("My Game"));
        assert_ne!(content, "ORIGINAL");

        let _ = fs::remove_dir_all(&dir);
    }
}
