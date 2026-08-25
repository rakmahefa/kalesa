use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use crate::domain::{LaunchOptions, Runner, RunnerKind};
use crate::error::{KalesaError, Result};
use crate::{GameMetadata, GameTarget};

pub const LAUNCHER_FORMAT_VERSION: u32 = 3;

pub fn write(
    path: &Path,
    config_path: &Path,
    target: &GameTarget,
    metadata: &GameMetadata,
    runner: &Runner,
    launch: &LaunchOptions,
) -> Result<()> {
    let config_path = config_path
        .to_str()
        .ok_or_else(|| KalesaError::InvalidDesktopValue("config path is not valid UTF-8".into()))?;

    launch.validate()?;

    let config_assignment = config_assignment(config_path);
    let target_value = shell_single_quote(&target.path.to_string_lossy());
    let name_value = shell_single_quote(&metadata.name);
    let runner_value = shell_single_quote(runner.kind.as_str());
    let binary_type_value = shell_single_quote(target.binary_type.as_str());
    let wine_prefix_value = runner
        .wine_prefix
        .as_ref()
        .map(|value| shell_single_quote(&value.to_string_lossy()))
        .unwrap_or_else(|| "''".to_string());
    let proton_path_value = runner
        .proton_path
        .as_ref()
        .map(|value| shell_single_quote(&value.to_string_lossy()))
        .unwrap_or_else(|| "''".to_string());

    let args = bash_array("CONFIG_ARGS", &launch.args);
    let wrappers = bash_array("CONFIG_WRAPPERS", &launch.wrappers);
    let env = bash_exports(&launch.env);

    let content = format!(
        r#"#!/bin/bash
# Generated launch script - do not edit by hand, re-run kalesa instead.
# Kalesa launcher format: {launcher_version}
# Kalesa config schema: 3
# Runtime values below are generated from {config_path}.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${{BASH_SOURCE[0]}}")" && pwd)"
BASE_DIR="$(dirname "$SCRIPT_DIR")"
GAME_DIR="$(dirname "$BASE_DIR")"
{config_assignment}

GAME_NAME={name}
BINARY_TYPE={binary_type}
RUNNER={runner}
TARGET_VALUE={target}
WINE_PREFIX_VALUE={wine_prefix}
PROTON_PATH_VALUE={proton_path}
{args}
{wrappers}
{env}

fail() {{
    echo "[!] $*" >&2
    exit 1
}}

resolve_path() {{
    local value="$1"
    if [[ "$value" = /* ]]; then
        printf '%s\n' "$value"
    else
        printf '%s\n' "$GAME_DIR/$value"
    fi
}}

require_command() {{
    local command_name="$1"
    command -v "$command_name" >/dev/null 2>&1 || fail "'$command_name' not found in PATH."
}}

require_wrapper() {{
    local wrapper="$1"
    if [[ "$wrapper" == */* ]]; then
        local resolved
        resolved="$(resolve_path "$wrapper")"
        [[ -x "$resolved" ]] || fail "wrapper is not executable: $resolved"
    else
        require_command "$wrapper"
    fi
}}

wrapper_command() {{
    local wrapper="$1"
    if [[ "$wrapper" == */* ]]; then
        resolve_path "$wrapper"
    else
        printf '%s\n' "$wrapper"
    fi
}}

print_command() {{
    local label="$1"
    shift
    local rendered
    rendered="$(printf ' %q' "$@")"
    echo "[+] $label${{rendered}}"
}}

cd "$GAME_DIR"

[[ -f "$CONFIG_FILE" ]] || fail "configuration not found: $CONFIG_FILE"
case "$BINARY_TYPE" in
    linux|appimage|windows) ;;
    *) fail "unsupported binary type: $BINARY_TYPE" ;;
esac

TARGET="$(resolve_path "$TARGET_VALUE")"
[[ -f "$TARGET" ]] || fail "game executable not found: $TARGET"

{runner_setup}

COMMAND=()
for wrapper in "${{CONFIG_WRAPPERS[@]}}"; do
    require_wrapper "$wrapper"
    COMMAND+=("$(wrapper_command "$wrapper")")
done

{runner_command}
COMMAND+=("${{CONFIG_ARGS[@]}}")
COMMAND+=("$@")

print_command "Launching $GAME_NAME:" "${{COMMAND[@]}}"
exec "${{COMMAND[@]}}"
"#,
        launcher_version = LAUNCHER_FORMAT_VERSION,
        config_path = config_path,
        config_assignment = config_assignment,
        name = name_value,
        binary_type = binary_type_value,
        runner = runner_value,
        target = target_value,
        wine_prefix = wine_prefix_value,
        proton_path = proton_path_value,
        args = args,
        wrappers = wrappers,
        env = env,
        runner_setup = runner_setup(runner.kind),
        runner_command = runner_command(runner.kind),
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

fn runner_setup(kind: RunnerKind) -> &'static str {
    match kind {
        RunnerKind::Native => r#"if [[ ! -x "$TARGET" ]]; then
    chmod +x "$TARGET" 2>/dev/null || true
fi
[[ -x "$TARGET" ]] || fail "game executable is not executable: $TARGET""#,
        RunnerKind::Wine => r#"require_command wine
[[ -n "$WINE_PREFIX_VALUE" ]] || fail "runner.wine.prefix is missing"
export WINEPREFIX="$(resolve_path "$WINE_PREFIX_VALUE")"
export WINEARCH="win64""#,
        RunnerKind::Proton => r#"[[ -n "$PROTON_PATH_VALUE" ]] || fail "runner.proton.path is missing"
[[ -n "$WINE_PREFIX_VALUE" ]] || fail "runner.wine.prefix is missing"
PROTON="$(resolve_path "$PROTON_PATH_VALUE")"
export WINEPREFIX="$(resolve_path "$WINE_PREFIX_VALUE")"
[[ -x "$PROTON" ]] || fail "Proton executable not found or not executable: $PROTON""#,
    }
}

fn runner_command(kind: RunnerKind) -> &'static str {
    match kind {
        RunnerKind::Native => r#"COMMAND+=("$TARGET")"#,
        RunnerKind::Wine => r#"require_command wine
COMMAND+=("wine" "$TARGET")"#,
        RunnerKind::Proton => r#"COMMAND+=("$PROTON" "run" "$TARGET")"#,
    }
}

fn bash_array(name: &str, values: &[String]) -> String {
    let mut output = format!("{name}=()\n");
    for value in values {
        output.push_str(&format!("{name}+=({})\n", shell_single_quote(value)));
    }
    output
}

fn bash_exports(values: &std::collections::BTreeMap<String, String>) -> String {
    let mut output = String::new();
    for (key, value) in values {
        output.push_str(&format!("export {key}={}\n", shell_single_quote(value)));
    }
    output
}

fn config_assignment(config_path: &str) -> String {
    let path = Path::new(config_path);
    if path.is_absolute() {
        format!("CONFIG_FILE={}", shell_single_quote(config_path))
    } else {
        format!(
            "CONFIG_FILE=\"$GAME_DIR/{}\"",
            shell_double_quote(config_path)
        )
    }
}

fn shell_single_quote(value: &str) -> String {
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
    use crate::domain::{BinaryType, GameMetadata, GameTarget, LaunchOptions, Runner, RunnerBackend};
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn generates_schema_v3_yq_free_runtime_launcher() {
        let root = temp_path("kalesa_launcher_v3");
        fs::create_dir_all(root.join("bin")).unwrap();
        let path = root.join("bin/launch.sh");
        let config = PathBuf::from(".workdir/config/config.yaml");
        let target = GameTarget::new(
            PathBuf::from("/games/My Game/game.exe"),
            BinaryType::WindowsPe,
        );
        let metadata = GameMetadata::new("My Game".into(), None);
        let project_dir = PathBuf::from("/games/My Game");
        let runner = Runner::for_target_with_backend(
            &target,
            &project_dir,
            RunnerBackend::Wine,
            None,
            None,
        );
        let mut env = BTreeMap::new();
        env.insert("WINEDEBUG".into(), "-all".into());
        let launch = LaunchOptions {
            args: vec!["hello world".into(), "--fullscreen".into()],
            env,
            wrappers: vec!["gamemoderun".into(), "mangohud".into()],
        };

        write(&path, &config, &target, &metadata, &runner, &launch).unwrap();
        let content = fs::read_to_string(&path).unwrap();

        assert!(content.contains("Kalesa launcher format: 3"));
        assert!(content.contains("Kalesa config schema: 3"));
        assert!(content.contains("CONFIG_ARGS+=('hello world')"));
        assert!(content.contains("CONFIG_WRAPPERS+=('gamemoderun')"));
        assert!(content.contains("export WINEDEBUG='-all'"));
        assert!(content.contains("COMMAND=()"));
        assert!(content.contains("COMMAND+=(\"wine\" \"$TARGET\")"));
        assert!(content.contains("echo \"[+] $label${rendered}\""));
        assert!(!content.contains("eval "));
        assert!(!content.contains("yq"));
        assert!(!content.contains("sh -c"));
    }

    #[test]
    fn shell_quote_handles_special_characters() {
        assert_eq!(shell_single_quote("a'b $HOME"), "'a'\\''b $HOME'");
    }

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("{name}_{}", std::process::id()))
    }
}
