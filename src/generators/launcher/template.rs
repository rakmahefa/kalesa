pub const TEMPLATE: &str = r##"#!/usr/bin/env bash
# Generated launch script - edit config.yaml, not this file.
# Kalesa launcher format: __KALESA_LAUNCHER_VERSION__
# Kalesa config schema: __KALESA_CONFIG_SCHEMA_VERSION__
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BASE_DIR="$(dirname "$SCRIPT_DIR")"
GAME_DIR="$(dirname "$BASE_DIR")"
CONFIG_FILE="$GAME_DIR/.workdir/config/config.yaml"

fail() {
    echo "[!] $*" >&2
    exit 1
}

require_command() {
    command -v "$1" >/dev/null 2>&1 || fail "required command not found: $1"
}

resolve_path() {
    local value="$1"
    if [[ "$value" = /* ]]; then
        printf '%s\n' "$value"
    else
        printf '%s\n' "$GAME_DIR/$value"
    fi
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
    printf '[+] %s' "$label"
    printf ' %q' "$@"
    printf '\n'
}

load_config() {
    require_command yq
    [[ -f "$CONFIG_FILE" ]] || fail "configuration not found: $CONFIG_FILE"

    local schema_version
    schema_version="$(yq -r '.schema_version // ""' "$CONFIG_FILE")" ||
        fail "invalid YAML in $CONFIG_FILE"
    [[ "$schema_version" == "__KALESA_CONFIG_SCHEMA_VERSION__" ]] ||
        fail "unsupported config schema: ${schema_version:-missing}; expected __KALESA_CONFIG_SCHEMA_VERSION__"

    GAME_NAME="$(yq -r '.name // ""' "$CONFIG_FILE")" || fail "cannot read name from $CONFIG_FILE"
    TARGET_VALUE="$(yq -r '.executable.path // ""' "$CONFIG_FILE")" || fail "cannot read executable.path from $CONFIG_FILE"
    RUNNER="$(yq -r '.runner.type // ""' "$CONFIG_FILE")" || fail "cannot read runner.type from $CONFIG_FILE"
    WINE_PREFIX_VALUE="$(yq -r '.runner.wine.prefix // ""' "$CONFIG_FILE")" || fail "cannot read runner.wine.prefix from $CONFIG_FILE"
    WINE_ARCH_VALUE="$(yq -r '.runner.wine.arch // ""' "$CONFIG_FILE")" || fail "cannot read runner.wine.arch from $CONFIG_FILE"
    PROTON_PATH_VALUE="$(yq -r '.runner.proton.path // ""' "$CONFIG_FILE")" || fail "cannot read runner.proton.path from $CONFIG_FILE"

    CONFIG_ARGS=()
    while IFS= read -r value; do
        [[ -n "$value" ]] && CONFIG_ARGS+=("$value")
    done < <(yq -r '.launch.args[]? // empty' "$CONFIG_FILE") ||
        fail "cannot read launch.args from $CONFIG_FILE"

    CONFIG_WRAPPERS=()
    while IFS= read -r value; do
        [[ -n "$value" ]] && CONFIG_WRAPPERS+=("$value")
    done < <(yq -r '.launch.wrappers[]? // empty' "$CONFIG_FILE") ||
        fail "cannot read launch.wrappers from $CONFIG_FILE"

    ENV_KEYS=()
    ENV_VALUES=()
    while IFS=$'\t' read -r key value; do
        [[ -n "$key" ]] || continue
        [[ "$key" =~ ^[a-zA-Z_][a-zA-Z0-9_]*$ ]] ||
            fail "invalid environment variable name in $CONFIG_FILE: $key"
        ENV_KEYS+=("$key")
        ENV_VALUES+=("$value")
    done < <(yq -r '.launch.env // {} | to_entries[]? | [.key, .value] | @tsv' "$CONFIG_FILE") ||
        fail "cannot read launch.env from $CONFIG_FILE"

    [[ -n "$GAME_NAME" ]] || fail "name is missing in $CONFIG_FILE"
    [[ -n "$TARGET_VALUE" ]] || fail "executable.path is missing in $CONFIG_FILE"

    case "$RUNNER" in
        native)
            ;;
        wine|proton)
            [[ -n "$WINE_PREFIX_VALUE" ]] || fail "runner.wine.prefix is missing in $CONFIG_FILE"
            [[ -n "$WINE_ARCH_VALUE" ]] || fail "runner.wine.arch is missing in $CONFIG_FILE"
            ;;
        *)
            fail "unsupported runner.type '$RUNNER' in $CONFIG_FILE"
            ;;
    esac

    if [[ "$RUNNER" == "proton" && -z "$PROTON_PATH_VALUE" ]]; then
        fail "runner.proton.path is missing in $CONFIG_FILE"
    fi
}

export_config_environment() {
    local i
    for i in "${!ENV_KEYS[@]}"; do
        export "${ENV_KEYS[$i]}=${ENV_VALUES[$i]}"
    done
}

load_config
export_config_environment
cd "$GAME_DIR"

TARGET="$(resolve_path "$TARGET_VALUE")"
[[ -f "$TARGET" ]] || fail "game executable not found: $TARGET"

COMMAND=()
for wrapper in "${CONFIG_WRAPPERS[@]}"; do
    require_wrapper "$wrapper"
    COMMAND+=("$(wrapper_command "$wrapper")")
done

case "$RUNNER" in
    native)
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
