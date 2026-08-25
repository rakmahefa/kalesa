use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use crate::domain::{LaunchOptions, Runner};
use crate::error::{KalesaError, Result};
use crate::{GameMetadata, GameTarget};

pub const LAUNCHER_FORMAT_VERSION: u32 = 4;

pub fn write(
    path: &Path,
    _config_path: &Path,
    _target: &GameTarget,
    _metadata: &GameMetadata,
    _runner: &Runner,
    _launch: &LaunchOptions,
) -> Result<()> {
    let kalesa_path = env::current_exe()
        .map_err(|e| KalesaError::io("resolving Kalesa runtime executable", e))?;
    let kalesa_path = kalesa_path.to_str().ok_or_else(|| {
        KalesaError::InvalidDesktopValue("Kalesa runtime executable path is not valid UTF-8".into())
    })?;
    let kalesa_binary = shell_single_quote(kalesa_path);

    let content = format!(
        r#"#!/bin/bash
# Generated launch script - do not edit by hand, re-run kalesa instead.
# Kalesa launcher format: {launcher_version}
# Kalesa config schema: 3
# The YAML manifest is parsed by the Kalesa Rust runtime; this script is only a stable launcher facade.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${{BASH_SOURCE[0]}}")" && pwd)"
BASE_DIR="$(dirname "$SCRIPT_DIR")"
GAME_DIR="$(dirname "$BASE_DIR")"
CONFIG_FILE="$GAME_DIR/.workdir/config/config.yaml"
KALESA_BIN={kalesa_binary}

fail() {{
    echo "[!] $*" >&2
    exit 1
}}

[[ -f "$CONFIG_FILE" ]] || fail "configuration not found: $CONFIG_FILE"

if [[ -x "$KALESA_BIN" ]]; then
    exec "$KALESA_BIN" --launch-config "$CONFIG_FILE" "$@"
fi

if command -v kalesa >/dev/null 2>&1; then
    exec "$(command -v kalesa)" --launch-config "$CONFIG_FILE" "$@"
fi

fail "Kalesa runtime executable not found. Expected '$KALESA_BIN' or a 'kalesa' command in PATH."
"#,
        launcher_version = LAUNCHER_FORMAT_VERSION,
        kalesa_binary = kalesa_binary,
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

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{BinaryType, GameMetadata, GameTarget, LaunchOptions, Runner, RunnerBackend};
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn generates_thin_schema_v3_runtime_launcher() {
        let root = temp_path("kalesa_launcher_v4");
        fs::create_dir_all(root.join("bin")).unwrap();
        let path = root.join("bin/launch.sh");
        let config = root.join(".workdir/config/config.yaml");
        fs::create_dir_all(config.parent().unwrap()).unwrap();
        let target = GameTarget::new(PathBuf::from("/games/My Game/game.exe"), BinaryType::WindowsPe);
        let metadata = GameMetadata::new("My Game".into(), None);
        let runner = Runner::for_target_with_backend(
            &target,
            Path::new("/games/My Game"),
            RunnerBackend::Wine,
            None,
            None,
        );
        let launch = LaunchOptions {
            args: vec!["hello world".into()],
            env: BTreeMap::from([("WINEDEBUG".into(), "-all".into())]),
            wrappers: vec!["gamemoderun".into()],
        };

        write(&path, &config, &target, &metadata, &runner, &launch).unwrap();
        let content = fs::read_to_string(&path).unwrap();

        assert!(content.contains("Kalesa launcher format: 4"));
        assert!(content.contains("CONFIG_FILE=\"$GAME_DIR/.workdir/config/config.yaml\""));
        assert!(content.contains("--launch-config \"$CONFIG_FILE\""));
        assert!(!content.contains("awk"));
        assert!(!content.contains("yq"));
        assert!(!content.contains("CONFIG_ARGS="));
        assert!(!content.contains("CONFIG_WRAPPERS="));
        assert!(!content.contains("WINE_PREFIX_VALUE="));
        assert!(!content.contains("eval "));
        assert!(!content.contains("sh -c"));
    }

    #[test]
    fn shell_quote_handles_single_quotes() {
        assert_eq!(shell_single_quote("a'b"), "'a'\\''b'");
    }

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("{name}_{}", std::process::id()))
    }
}
