use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use crate::domain::{LaunchOptions, Runner};
use crate::error::{KalesaError, Result};
use crate::{GameMetadata, GameTarget};

pub const LAUNCHER_FORMAT_VERSION: u32 = 3;

pub fn write(
    path: &Path,
    _config_path: &Path,
    _target: &GameTarget,
    _metadata: &GameMetadata,
    _runner: &Runner,
    launch: &LaunchOptions,
) -> Result<()> {
    launch.validate()?;

    let content =
        LAUNCHER_TEMPLATE.replace("__KalesaVersion__", &LAUNCHER_FORMAT_VERSION.to_string());

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

const LAUNCHER_TEMPLATE: &str = r##"#!/usr/bin/env bash
# Generated launch script - do not edit by hand, re-run kalesa instead.
# Kalesa launcher format: __KalesaVersion__
# Kalesa config schema: 3
# Runtime configuration is read from $GAME_DIR/.workdir/config/config.yaml.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BASE_DIR="$(dirname "$SCRIPT_DIR")"
GAME_DIR="$(dirname "$BASE_DIR")"
CONFIG_FILE="$GAME_DIR/.workdir/config/config.yaml"

SCHEMA_VERSION=
GAME_NAME=
RUNNER=
TARGET_VALUE=
WINE_PREFIX_VALUE=
WINE_ARCH_VALUE=
PROTON_PATH_VALUE=

CONFIG_ARGS=()
CONFIG_WRAPPERS=()
ENV_KEYS=()
ENV_VALUES=()

fail() {
    echo "[!] $*" >&2
    exit 1
}

resolve_path() {
    local value="$1"
    if [[ "$value" = /* ]]; then
        printf '%s\n' "$value"
    else
        printf '%s\n' "$GAME_DIR/$value"
    fi
}

validate_env_key() {
    local key="$1"
    [[ "$key" =~ ^[a-zA-Z_][a-zA-Z0-9_]*$ ]] ||
        fail "invalid environment variable name in config.yaml: $key"
}

load_config() {
    local parsed record kind value

    [[ -f "$CONFIG_FILE" ]] ||
        fail "configuration not found: $CONFIG_FILE"

    parsed="$(
        awk '
            BEGIN {
                dq = sprintf("%c", 34)
                sq = sprintf("%c", 39)
            }

            function trim(value) {
                sub(/^[[:space:]]+/, "", value)
                sub(/[[:space:]]+$/, "", value)
                return value
            }

            function strip_comment(value,    i, c, quote, escaped, out) {
                out = ""
                quote = ""
                escaped = 0
                for (i = 1; i <= length(value); i++) {
                    c = substr(value, i, 1)
                    if (quote == dq) {
                        out = out c
                        if (escaped) {
                            escaped = 0
                        } else if (c == "\\") {
                            escaped = 1
                        } else if (c == dq) {
                            quote = ""
                        }
                    } else if (quote == sq) {
                        out = out c
                        if (c == sq) {
                            if (substr(value, i + 1, 1) == sq) {
                                out = out sq
                                i++
                            } else {
                                quote = ""
                            }
                        }
                    } else if (c == dq || c == sq) {
                        quote = c
                        out = out c
                    } else if (c == "#") {
                        break
                    } else {
                        out = out c
                    }
                }
                return trim(out)
            }

            function scalar(value,    first, last, body, i, c, escaped, next_char, out) {
                value = trim(value)
                if (value == "" || value == "null" || value == "~") {
                    return ""
                }

                first = substr(value, 1, 1)
                last = substr(value, length(value), 1)

                if (first == sq && last == sq) {
                    body = substr(value, 2, length(value) - 2)
                    gsub(sq sq, sq, body)
                    return body
                }

                if (first == dq && last == dq) {
                    body = substr(value, 2, length(value) - 2)
                    out = ""
                    escaped = 0
                    for (i = 1; i <= length(body); i++) {
                        c = substr(body, i, 1)
                        if (escaped) {
                            next_char = c
                            if (next_char == "n") out = out "\n"
                            else if (next_char == "r") out = out "\r"
                            else if (next_char == "t") out = out "\t"
                            else out = out next_char
                            escaped = 0
                        } else if (c == "\\") {
                            escaped = 1
                        } else {
                            out = out c
                        }
                    }
                    if (escaped) out = out "\\"
                    return out
                }

                return value
            }

            function key_pos(line,    i, c, quote) {
                quote = ""
                for (i = 1; i <= length(line); i++) {
                    c = substr(line, i, 1)
                    if (c == dq || c == sq) {
                        if (quote == "") quote = c
                        else if (quote == c) quote = ""
                    } else if (c == ":" && quote == "") {
                        return i
                    }
                }
                return 0
            }

            function current_path(    i, out) {
                out = ""
                for (i = 1; i <= depth; i++) {
                    out = (out == "" ? "" : out ".") keys[i]
                }
                return out
            }

            function emit_scalar(path, value) {
                printf "S\t%s\t%s\n", path, scalar(value)
            }

            {
                sub(/\r$/, "", $0)
                line = strip_comment($0)
                if (line == "") next

                indent = 0
                while (substr(line, indent + 1, 1) == " ") indent++
                text = substr(line, indent + 1)

                if (substr(text, 1, 2) == "- ") {
                    item = scalar(substr(text, 3))
                    parent = current_path()
                    if (parent == "launch.args") {
                        printf "A\t%s\n", item
                        next
                    }
                    if (parent == "launch.wrappers") {
                        printf "W\t%s\n", item
                        next
                    }
                    exit 2
                }

                pos = key_pos(text)
                if (pos == 0) exit 2

                while (depth > 0 && indent <= indents[depth]) depth--
                depth++
                indents[depth] = indent
                keys[depth] = trim(substr(text, 1, pos - 1))
                value = substr(text, pos + 1)
                path = current_path()

                if (path == "schema_version" ||
                    path == "name" ||
                    path == "version" ||
                    path == "developer" ||
                    path == "description" ||
                    path == "runner.type" ||
                    path == "runner.wine.prefix" ||
                    path == "runner.wine.arch" ||
                    path == "runner.proton.path" ||
                    path == "executable.path") {
                    emit_scalar(path, value)
                    next
                }

                if (depth >= 3 && keys[1] == "launch" && keys[2] == "env") {
                    printf "E\t%s\t%s\n", keys[3], scalar(value)
                    next
                }

                if (path == "categories" ||
                    path == "runner.wine" ||
                    path == "runner.proton" ||
                    path == "launch.args" ||
                    path == "launch.env" ||
                    path == "launch.wrappers") {
                    next
                }

                if (value != "") exit 2
            }
        ' "$CONFIG_FILE"
    )" || fail "invalid or unsupported YAML in $CONFIG_FILE"

    while IFS=$'\t' read -r record kind value; do
        case "$record" in
            S)
                case "$kind" in
                    schema_version) SCHEMA_VERSION="$value" ;;
                    name) GAME_NAME="$value" ;;
                    runner.type) RUNNER="$value" ;;
                    runner.wine.prefix) WINE_PREFIX_VALUE="$value" ;;
                    runner.wine.arch) WINE_ARCH_VALUE="$value" ;;
                    runner.proton.path) PROTON_PATH_VALUE="$value" ;;
                    executable.path) TARGET_VALUE="$value" ;;
                esac
                ;;
            A)
                CONFIG_ARGS+=("$kind")
                ;;
            W)
                CONFIG_WRAPPERS+=("$kind")
                ;;
            E)
                validate_env_key "$kind"
                ENV_KEYS+=("$kind")
                ENV_VALUES+=("$value")
                ;;
            *)
                fail "invalid configuration record generated from $CONFIG_FILE"
                ;;
        esac
    done <<< "$parsed"

    [[ "$SCHEMA_VERSION" == "3" ]] ||
        fail "unsupported config schema: ${SCHEMA_VERSION:-missing}; expected 3"
    [[ -n "$GAME_NAME" ]] || fail "name is missing in $CONFIG_FILE"
    [[ -n "$RUNNER" ]] || fail "runner.type is missing in $CONFIG_FILE"
    [[ -n "$TARGET_VALUE" ]] || fail "executable.path is missing in $CONFIG_FILE"

    case "$RUNNER" in
        native)
            ;;
        wine|proton)
            [[ -n "$WINE_PREFIX_VALUE" ]] ||
                fail "runner.wine.prefix is missing in $CONFIG_FILE"
            [[ -n "$WINE_ARCH_VALUE" ]] ||
                fail "runner.wine.arch is missing in $CONFIG_FILE"
            ;;
        *)
            fail "unsupported runner.type '$RUNNER' in $CONFIG_FILE"
            ;;
    esac

    if [[ "$RUNNER" == "proton" ]]; then
        [[ -n "$PROTON_PATH_VALUE" ]] ||
            fail "runner.proton.path is missing in $CONFIG_FILE"
    fi
}

export_config_environment() {
    local i
    for i in "${!ENV_KEYS[@]}"; do
        export "${ENV_KEYS[$i]}=${ENV_VALUES[$i]}"
    done
}

require_wrapper() {
    local wrapper="$1"
    if [[ "$wrapper" == */* ]]; then
        local resolved
        resolved="$(resolve_path "$wrapper")"
        [[ -x "$resolved" ]] || fail "wrapper is not executable: $resolved"
    else
        require_command "$wrapper"
    fi
}

wrapper_command() {
    local wrapper="$1"
    if [[ "$wrapper" == */* ]]; then
        resolve_path "$wrapper"
    else
        printf '%s\n' "$wrapper"
    fi
}

print_command() {
    local label="$1"
    shift
    local rendered
    rendered="$(printf ' %q' "$@")"
    echo "[+] $label${rendered}"
}

cd "$GAME_DIR"
load_config
export_config_environment

TARGET="$(resolve_path "$TARGET_VALUE")"
[[ -f "$TARGET" ]] || fail "game executable not found: $TARGET"

COMMAND=()
for wrapper in "${CONFIG_WRAPPERS[@]}"; do
    require_wrapper "$wrapper"
    COMMAND+=("$(wrapper_command "$wrapper")")
done

case "$RUNNER" in
    native)
        if [[ ! -x "$TARGET" ]]; then
            chmod +x "$TARGET" 2>/dev/null || true
        fi
        [[ -x "$TARGET" ]] || fail "game executable is not executable: $TARGET"
        COMMAND+=("$TARGET")
        ;;
    wine)
        require_command wine
        export WINEPREFIX="$(resolve_path "$WINE_PREFIX_VALUE")"
        export WINEARCH="$WINE_ARCH_VALUE"
        COMMAND+=("wine" "$TARGET")
        ;;
    proton)
        PROTON="$(resolve_path "$PROTON_PATH_VALUE")"
        [[ -x "$PROTON" ]] || fail "Proton executable not found or not executable: $PROTON"
        export WINEPREFIX="$(resolve_path "$WINE_PREFIX_VALUE")"
        export WINEARCH="$WINE_ARCH_VALUE"
        COMMAND+=("$PROTON" "run" "$TARGET")
        ;;
esac

COMMAND+=("${CONFIG_ARGS[@]}")
COMMAND+=("$@")

print_command "Launching $GAME_NAME:" "${COMMAND[@]}"
exec "${COMMAND[@]}"
"##;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{BinaryType, GameTarget, LaunchOptions, RunnerBackend};
    use crate::GameMetadata;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn generated_launcher_is_yaml_driven() {
        let root = temp_path("kalesa_launcher_yaml");
        fs::create_dir_all(root.join(".workdir/bin")).unwrap();
        let launcher = root.join(".workdir/bin/launch.sh");
        let target = GameTarget::new(PathBuf::from("/ignored/game.exe"), BinaryType::WindowsPe);
        let runner =
            Runner::for_target_with_backend(&target, &root, RunnerBackend::Wine, None, None);

        write(
            &launcher,
            &root.join(".workdir/config/config.yaml"),
            &target,
            &GameMetadata::new("Ignored".into(), None),
            &runner,
            &LaunchOptions::default(),
        )
        .unwrap();

        let content = fs::read_to_string(launcher).unwrap();
        assert!(content.contains("CONFIG_FILE=\"$GAME_DIR/.workdir/config/config.yaml\""));
        assert!(content.contains("load_config()"));
        assert!(content.contains("runner.wine.prefix"));
        assert!(content.contains("runner.proton.path"));
        assert!(content.contains("launch.args"));
        assert!(content.contains("launch.wrappers"));
        assert!(content.contains("launch.env"));
        assert!(!content.contains("/ignored/game.exe"));
        assert!(!content.contains("eval "));
        assert!(!content.contains("sh -c"));
        assert!(!content.contains("yq"));
    }

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("{name}_{}", std::process::id()))
    }
}
