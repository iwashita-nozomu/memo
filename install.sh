#!/usr/bin/env bash

set -euo pipefail

script_path="$(readlink -f -- "${BASH_SOURCE[0]}")"
script_dir="$(cd -- "$(dirname -- "$script_path")" && pwd -P)"
destination="$HOME/.local/bin/memo"
config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/memo"
config_path="$config_dir/config.toml"

if ! command -v cargo >/dev/null 2>&1; then
    printf 'memo installer: cargo is required to compile the CLI\n' >&2
    exit 1
fi

if [[ -L "$destination" ]]; then
    rm -- "$destination"
fi
mkdir -p -- "$(dirname -- "$destination")"

printf '==> Compile memo CLI\n'
cargo install --path "$script_dir" --root "$HOME/.local" --force

mkdir -p -- "$config_dir"
if [[ ! -e "$config_path" ]]; then
    cp -- "$script_dir/config.example.toml" "$config_path"
    printf 'memo config template: %s\n' "$config_path"
else
    printf 'memo config preserved: %s\n' "$config_path"
fi
printf 'memo command: %s\n' "$destination"
