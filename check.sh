#!/usr/bin/env bash

set -euo pipefail

destination="$HOME/.local/bin/memo"
config_path="${MEMO_CONFIG:-${XDG_CONFIG_HOME:-$HOME/.config}/memo/config.toml}"

if [[ ! -x "$destination" ]]; then
    printf 'memo command is missing: %s\n' "$destination" >&2
    exit 1
fi

version="$("$destination" --version)"
if [[ ! -f "$config_path" ]]; then
    printf 'version=%s config=not-configured (%s)\n' "$version" "$config_path"
    exit 0
fi

check_file="$(mktemp "${TMPDIR:-/tmp}/dotfiles-memo-check.XXXXXX")"
trap 'rm -f -- "$check_file"' EXIT
if "$destination" --check-config >"$check_file" 2>&1; then
    output="$(cat "$check_file")"
    printf 'version=%s %s\n' "$version" "${output//$'\n'/ }"
else
    output="$(cat "$check_file" 2>/dev/null || true)"
    printf 'version=%s config=not-ready (%s): %s\n' "$version" "$config_path" "$output" >&2
fi
