use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use crate::error::{KalesaError, Result};

pub const LAUNCHER_FORMAT_VERSION: u32 = 2;

pub fn write(path: &Path, config_path: &Path) -> Result<()> {
    let config_path = config_path
        .to_str()
        .ok_or_else(|| KalesaError::InvalidDesktopValue("config path is not valid UTF-8".into()))?;
    let config_assignment = config_assignment(config_path);

    let content = format!(
        r#"#!/bin/bash
# Generated launch script - do not edit by hand, re-run kalesa instead.
# Kalesa launcher format: {launcher_version}
# Kalesa config schema: 2
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${{BASH_SOURCE[0]}}")" && pwd)"
BASE_DIR="$(dirname "$SCRIPT_DIR")"
GAME_DIR="$(dirname "$BASE_DIR")"
{config_assignment}
YQ_BIN="${{KALESA_YQ:-yq}}"

fail() {{
    echo "[!] $*" >&2
    exit 1
}}

require_command() {{
    command -v "$1" >/dev/null 2>&1 || fail "'$1' not found in PATH. Install it to use this launcher."
}}

resolve_path() {{
    local value="$1"
    if [[ "$value" = /* ]]; then
        printf '%s\n' "$value"
    else
        printf '%s\n' "$GAME_DIR/$value"
    fi
}}

load_config_args() {{
    CONFIG_ARGS=()
    while IFS= read -r encoded; do
        [[ -n "$encoded" ]] || continue
        eval "CONFIG_ARGS+=( $encoded )"
    done < <("$YQ_BIN" -r '.launch.args[]? | @sh' "$CONFIG_FILE")
}}

load_config_env() {{
    while IFS= read -r encoded; do
        [[ -n "$encoded" ]] || continue
        ENV_PAIR=()
        eval "ENV_PAIR=( $encoded )"
        [[ "${{#ENV_PAIR[@]}}" -eq 2 ]] || fail "invalid launch.env entry in $CONFIG_FILE"
        export "${{ENV_PAIR[0]}}=${{ENV_PAIR[1]}}"
    done < <("$YQ_BIN" -r '.launch.env[]? | [.key, .value] | @sh' "$CONFIG_FILE")
}}

cd "$GAME_DIR"

[[ -f "$CONFIG_FILE" ]] || fail "configuration not found: $CONFIG_FILE"
require_command "$YQ_BIN"

SCHEMA_VERSION=$("$YQ_BIN" -r '.schema_version // 0' "$CONFIG_FILE")
[[ "$SCHEMA_VERSION" == "2" ]] || fail "unsupported config schema: $SCHEMA_VERSION (expected 2)"

NAME=$("$YQ_BIN" -r '.name // "Unknown game"' "$CONFIG_FILE")
RUNNER=$("$YQ_BIN" -r '.runner.type // ""' "$CONFIG_FILE")
TARGET_VALUE=$("$YQ_BIN" -r '.executable.path // ""' "$CONFIG_FILE")
[[ -n "$RUNNER" ]] || fail "runner.type is missing from $CONFIG_FILE"
[[ -n "$TARGET_VALUE" ]] || fail "executable.path is missing from $CONFIG_FILE"

TARGET=$(resolve_path "$TARGET_VALUE")
[[ -f "$TARGET" ]] || fail "game executable not found: $TARGET"

load_config_args
load_config_env

case "$RUNNER" in
    native)
        if [[ ! -x "$TARGET" ]]; then
            chmod +x "$TARGET" 2>/dev/null || true
        fi
        [[ -x "$TARGET" ]] || fail "game executable is not executable: $TARGET"
        echo "[+] Launching $NAME natively..."
        exec "$TARGET" "${{CONFIG_ARGS[@]}}" "$@"
        ;;

    wine)
        require_command wine
        WINE_PREFIX_VALUE=$("$YQ_BIN" -r '.runner.wine.prefix // ""' "$CONFIG_FILE")
        WINE_ARCH=$("$YQ_BIN" -r '.runner.wine.arch // "win64"' "$CONFIG_FILE")
        [[ -n "$WINE_PREFIX_VALUE" ]] || fail "runner.wine.prefix is missing from $CONFIG_FILE"
        export WINEPREFIX="$(resolve_path "$WINE_PREFIX_VALUE")"
        export WINEARCH="$WINE_ARCH"
        echo "[+] Launching $NAME via Wine..."
        exec wine "$TARGET" "${{CONFIG_ARGS[@]}}" "$@"
        ;;

    proton)
        PROTON_VALUE=$("$YQ_BIN" -r '.runner.proton.path // ""' "$CONFIG_FILE")
        WINE_PREFIX_VALUE=$("$YQ_BIN" -r '.runner.wine.prefix // ""' "$CONFIG_FILE")
        [[ -n "$PROTON_VALUE" ]] || fail "runner.proton.path is missing from $CONFIG_FILE"
        [[ -n "$WINE_PREFIX_VALUE" ]] || fail "runner.wine.prefix is missing from $CONFIG_FILE"
        PROTON=$(resolve_path "$PROTON_VALUE")
        export WINEPREFIX="$(resolve_path "$WINE_PREFIX_VALUE")"
        [[ -x "$PROTON" ]] || fail "Proton executable not found or not executable: $PROTON"
        echo "[+] Launching $NAME via Proton..."
        exec "$PROTON" run "$TARGET" "${{CONFIG_ARGS[@]}}" "$@"
        ;;

    *)
        fail "unsupported runner.type '$RUNNER' in $CONFIG_FILE"
        ;;
esac
"#,
        launcher_version = LAUNCHER_FORMAT_VERSION,
        config_assignment = config_assignment,
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

fn config_assignment(config_path: &str) -> String {
    let path = Path::new(config_path);
    if path.is_absolute() {
        format!("CONFIG_FILE={}", shell_quote(config_path))
    } else {
        format!("CONFIG_FILE=\"$GAME_DIR/{}\"", shell_double_quote(config_path))
    }
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn shell_double_quote(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('$', "\\$")
        .replace('`', "\\`")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn generates_schema_v2_runtime_launcher() {
        let root = temp_path("kalesa_launcher_v2");
        fs::create_dir_all(root.join("bin")).unwrap();
        let path = root.join("bin/launch.sh");
        let config = PathBuf::from(".workdir/config/config.yaml");

        write(&path, &config).unwrap();
        let content = fs::read_to_string(&path).unwrap();

        assert!(content.contains("Kalesa launcher format: 2"));
        assert!(content.contains("Kalesa config schema: 2"));
        assert!(content.contains("CONFIG_FILE=\"$GAME_DIR/.workdir/config/config.yaml\""));
        assert!(content.contains(".schema_version // 0"));
        assert!(content.contains(".runner.type // \"\""));
        assert!(content.contains(".executable.path // \"\""));
        assert!(content.contains(".launch.args[]? | @sh"));
        assert!(content.contains(".launch.env[]? | [.key, .value] | @sh"));
        assert!(content.contains("exec wine \"$TARGET\" \"${CONFIG_ARGS[@]}\" \"$@\""));
    }

    #[test]
    fn quotes_relative_config_path() {
        let root = temp_path("kalesa_launcher_config_quote");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("launch.sh");
        let config = PathBuf::from(".workdir/config/game 'file.yaml");

        write(&path, &config).unwrap();
        let content = fs::read_to_string(&path).unwrap();

        assert!(content.contains(
            "CONFIG_FILE=\"$GAME_DIR/.workdir/config/game \\\'file.yaml\""
        ));
    }

    #[test]
    fn quotes_absolute_config_path() {
        let root = temp_path("kalesa_launcher_absolute_quote");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("launch.sh");
        let config = root.join("config 'file.yaml");

        write(&path, &config).unwrap();
        let content = fs::read_to_string(&path).unwrap();
        let expected = format!("CONFIG_FILE={}
", shell_quote(config.to_str().unwrap()));

        assert!(content.contains(expected.trim_end()));
    }

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("{name}_{}", std::process::id()))
    }
}
