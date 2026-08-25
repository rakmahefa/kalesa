use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use crate::domain::{GameTarget, Runner, RunnerKind};
use crate::error::{KalesaError, Result};

fn shell_quote(value: &Path) -> Result<String> {
    let value = value
        .to_str()
        .ok_or_else(|| KalesaError::InvalidDesktopValue("path is not valid UTF-8".into()))?;
    Ok(format!("'{}'", value.replace('\'', "'\\''")))
}

pub fn write(path: &Path, target: &GameTarget, runner: &Runner) -> Result<()> {
    let executable = shell_quote(&target.path)?;
    let run_block = match runner.kind {
        RunnerKind::Wine => {
            let prefix = runner
                .wine_prefix
                .as_deref()
                .ok_or(KalesaError::MissingWinePrefix)?;
            let prefix = shell_quote(prefix)?;
            format!(
                "if ! command -v wine >/dev/null 2>&1; then\n    echo \"[!] 'wine' not found in PATH. Install wine to launch this Windows game.\" >&2\n    exit 1\nfi\n\nexport WINEPREFIX={prefix}\nexport WINEARCH=win64\n\necho \"[+] Launching Windows game via Wine...\"\nexec wine {executable} \"$@\"\n"
            )
        }
        RunnerKind::Native => format!(
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
        assert_eq!(shell_quote(path).unwrap(), r#"'/tmp/My Game'\''s [1998].exe'"#);
    }

    #[test]
    fn windows_launch_script_uses_safe_quoting() {
        let dir = temp_dir("win");
        let output = dir.join("launch.sh");
        let target = GameTarget::new(
            Path::new("/tmp/My Game's.exe").to_path_buf(),
            crate::domain::BinaryType::WindowsPe,
        );
        let runner = Runner::for_target(&target, &dir);

        write(&output, &target, &runner).unwrap();
        let content = fs::read_to_string(&output).unwrap();

        assert!(content.contains("exec wine '/tmp/My Game'\\''s.exe'"));
        assert!(content.contains("WINEPREFIX='"));
        assert!(content.starts_with("#!/bin/bash"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn linux_launch_script_runs_absolute_target() {
        let dir = temp_dir("linux");
        let output = dir.join("launch.sh");
        let target = GameTarget::new(
            Path::new("/tmp/My Game's").to_path_buf(),
            crate::domain::BinaryType::LinuxElf,
        );
        let runner = Runner::for_target(&target, &dir);

        write(&output, &target, &runner).unwrap();
        let content = fs::read_to_string(&output).unwrap();

        assert!(content.contains("TARGET='/tmp/My Game'\\''s'"));
        assert!(!content.contains("exec wine"));

        let _ = fs::remove_dir_all(&dir);
    }
}
