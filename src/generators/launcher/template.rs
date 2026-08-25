pub const TEMPLATE: &str = r##"#!/usr/bin/env bash
# Generated launch script - do not edit by hand, re-run kalesa instead.
__KALESA_RUNTIME_VALUES__
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
    local rendered
    rendered="$(printf ' %q' "$@")"
    echo "[+] $label${rendered}"
}

[[ -f "$CONFIG_FILE" ]] || fail "configuration not found: $CONFIG_FILE"
[[ "$RUNNER" == "__RUNNER_COMMAND__" ]] || fail "generated runner state is inconsistent"

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
        if [[ ! -x "$TARGET" ]]; then
            chmod +x "$TARGET" 2>/dev/null || true
        fi
        [[ -x "$TARGET" ]] || fail "game executable is not executable: $TARGET"
        COMMAND+=("$TARGET")
        ;;
    wine)
        require_command wine
        [[ -n "$WINE_PREFIX_VALUE" ]] || fail "Wine prefix is missing"
        export WINEPREFIX="$(resolve_path "$WINE_PREFIX_VALUE")"
        export WINEARCH="$WINE_ARCH_VALUE"
        COMMAND+=("wine" "$TARGET")
        ;;
    proton)
        require_command bash
        [[ -n "$PROTON_PATH_VALUE" ]] || fail "Proton executable path is missing"
        PROTON="$(resolve_path "$PROTON_PATH_VALUE")"
        [[ -x "$PROTON" ]] || fail "Proton executable not found or not executable: $PROTON"
        [[ -n "$WINE_PREFIX_VALUE" ]] || fail "Proton prefix is missing"
        export WINEPREFIX="$(resolve_path "$WINE_PREFIX_VALUE")"
        export WINEARCH="$WINE_ARCH_VALUE"
        COMMAND+=("$PROTON" "run" "$TARGET")
        ;;
    *)
        fail "unsupported generated runner: $RUNNER"
        ;;
esac

COMMAND+=("${CONFIG_ARGS[@]}")
COMMAND+=("$@")

print_command "Launching $GAME_NAME:" "${COMMAND[@]}"
exec "${COMMAND[@]}"
"##;
