use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use crate::domain::{GameTarget, LaunchOptions, Runner, RunnerKind};
use crate::error::{KalesaError, Result};

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn shell_quote_path(path: &Path) -> Result<String> {
    let value = path
        .to_str()
        .ok_or_else(|| KalesaError::InvalidDesktopValue("path is not valid UTF-8".into()))?;
    Ok(shell_quote(value))
}

pub fn write(
    path: &Path,
    target: &GameTarget,
    runner: &Runner,
    launch: &LaunchOptions,
) -> Result<()> {
    launch.validate()?;
    let executable = shell_quote_path(&target.path)?;
    let configured_args = launch
        .args
        .iter()
        .map(|arg| shell_quote(arg))
        .collect::<Vec<_>>()
        .join(" ");
    let env_exports = launch
        .env
        .iter()
        .map(|(key, value)| format!("export {key}={}", shell_quote(value)))
        .collect::<Vec<_>>()
        .join("\n");
    let arg_suffix = if configured_args.is_empty() {
        "\"$@\"".to_string()
    } else {
        format!("{configured_args} \"$@\"")
    };

    let run_block = match runner.kind {
        RunnerKind::Wine => {
            let prefix = runner
                .wine_prefix
                .as_deref()
                .ok_or(KalesaError::MissingWinePrefix)?;
            let prefix = shell_quote_path(prefix)?;
            format!(
                "if ! command -v wine >/dev/null 2>&1; then\n    echo \"[!] 'wine' not found in PATH. Install wine to launch this Windows game.\" >&2\n    exit 1\nfi\n\nexport WINEPREFIX={prefix}\nexport WINEARCH=win64\n{env_exports}\n\necho \"[+] Launching Windows game via Wine...\"\nexec wine {executable} {arg_suffix}\n"
            )
        }
        RunnerKind::Proton => {
            let proton = runner
                .proton_path
                .as_deref()
                .ok_or(KalesaError::MissingProtonPath)?;
            let proton = shell_quote_path(proton)?;
            let prefix = runner
                .wine_prefix
                .as_deref()
                .ok_or(KalesaError::MissingWinePrefix)?;
            let prefix = shell_quote_path(prefix)?;
            format!(
                "if [ ! -x {proton} ]; then\n    echo \"[!] Proton executable not found or not executable: {proton}\" >&2\n    exit 1\nfi\n\nexport WINEPREFIX={prefix}\n{env_exports}\n\necho \"[+] Launching Windows game via Proton...\"\nexec {proton} run {executable} {arg_suffix}\n"
            )
        }
        RunnerKind::Native => format!(
            "TARGET={executable}\nif [ ! -x \"$TARGET\" ]; then\n    chmod +x \"$TARGET\" 2>/dev/null || true\nfi\n{env_exports}\n\necho \"[+] Launching Linux game...\"\nexec \"$TARGET\" {arg_suffix}\n"
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
