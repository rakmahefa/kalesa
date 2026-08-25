use log::{info, warn};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

/// Generates `.workdir/bin/launch.sh`.
///
/// For Windows targets, the script launches the game through `wine`, using
/// the Wine prefix configured for this game (see `config::write_config`).
/// For Linux targets, the script executes the binary directly (marking it
/// executable first if needed).
pub fn write_launch_script(
    path: &Path,
    exe_filename: &str,
    is_windows: bool,
    wine_prefix: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let run_block = if is_windows {
        let prefix = wine_prefix.unwrap_or_default();
        format!(
            r#"if ! command -v wine >/dev/null 2>&1; then
    echo "[!] 'wine' not found in PATH. Install wine to launch this Windows game." >&2
    exit 1
fi

export WINEPREFIX="{prefix}"
export WINEARCH=win64

echo "[+] Launching {exe} via Wine..."
exec wine "$GAME_DIR/{exe}" "$@"
"#,
            prefix = prefix,
            exe = exe_filename
        )
    } else {
        format!(
            r#"TARGET="$GAME_DIR/{exe}"
if [ ! -x "$TARGET" ]; then
    chmod +x "$TARGET" 2>/dev/null || true
fi

echo "[+] Launching {exe}..."
exec "$TARGET" "$@"
"#,
            exe = exe_filename
        )
    };

    let content = format!(
        r#"#!/bin/bash
# Generated launch script - do not edit by hand, re-run kalesa instead.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${{BASH_SOURCE[0]}}")" && pwd)"
BASE_DIR="$(dirname "$SCRIPT_DIR")"
GAME_DIR="$(dirname "$BASE_DIR")"

cd "$GAME_DIR"

if [ ! -f ".workdir/config/config.yaml" ]; then
    echo "[!] .workdir/config/config.yaml not found - setup may be incomplete." >&2
    exit 1
fi

{run_block}"#,
        run_block = run_block
    );

    let mut file = File::create(path)?;
    file.write_all(content.as_bytes())?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755))?;
    }

    Ok(())
}

/// Writes `game.desktop` and `.directory` inside `output_dir`.
///
/// If `force` is `false` and a file already exists, it is left untouched (a
/// warning is logged) so re-running the tool doesn't silently clobber a
/// user's manual customizations. Pass `force: true` to always overwrite.
pub fn write_desktop_entries(
    game_name: &str,
    current_dir_str: &str,
    icon_path: &str,
    output_dir: &Path,
    force: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let desktop_content = format!(
        r#"[Desktop Entry]
Type=Application
Name={}
Exec={}/.workdir/bin/launch.sh
Icon={}
Terminal=false
Categories=Game;
"#,
        game_name, current_dir_str, icon_path
    );
    write_if_allowed(&output_dir.join("game.desktop"), &desktop_content, force)?;

    let directory_content = format!(
        r#"[Desktop Entry]
Type=Directory
Name={}
Icon={}
"#,
        game_name, icon_path
    );
    write_if_allowed(&output_dir.join(".directory"), &directory_content, force)?;

    Ok(())
}

fn write_if_allowed(path: &Path, content: &str, force: bool) -> std::io::Result<()> {
    if path.exists() && !force {
        warn!(
            "{:?} already exists, skipping (pass --force to overwrite)",
            path
        );
        return Ok(());
    }
    let mut file = File::create(path)?;
    file.write_all(content.as_bytes())?;
    info!("Generated {:?}", path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(label: &str) -> std::path::PathBuf {
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
    fn launch_script_windows_uses_wine() {
        let dir = temp_dir("win");
        let path = dir.join("launch.sh");

        write_launch_script(&path, "game.exe", true, Some("/tmp/wineprefix")).unwrap();
        let content = fs::read_to_string(&path).unwrap();

        assert!(content.contains("exec wine"));
        assert!(content.contains(r#"WINEPREFIX="/tmp/wineprefix""#));
        assert!(content.contains("game.exe"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn launch_script_linux_runs_directly() {
        let dir = temp_dir("linux");
        let path = dir.join("launch.sh");

        write_launch_script(&path, "game.bin", false, None).unwrap();
        let content = fs::read_to_string(&path).unwrap();

        assert!(!content.contains("wine"));
        assert!(content.contains("game.bin"));
        assert!(content.contains("chmod +x"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn desktop_entries_are_not_overwritten_without_force() {
        let dir = temp_dir("desktop_noforce");
        fs::write(dir.join("game.desktop"), "ORIGINAL").unwrap();

        write_desktop_entries("My Game", "/tmp/mygame", "applications-games", &dir, false)
            .unwrap();

        let content = fs::read_to_string(dir.join("game.desktop")).unwrap();
        assert_eq!(content, "ORIGINAL");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn desktop_entries_are_overwritten_with_force() {
        let dir = temp_dir("desktop_force");
        fs::write(dir.join("game.desktop"), "ORIGINAL").unwrap();

        write_desktop_entries("My Game", "/tmp/mygame", "applications-games", &dir, true)
            .unwrap();

        let content = fs::read_to_string(dir.join("game.desktop")).unwrap();
        assert!(content.contains("My Game"));
        assert_ne!(content, "ORIGINAL");

        let _ = fs::remove_dir_all(&dir);
    }
}
