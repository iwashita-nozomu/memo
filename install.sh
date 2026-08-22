#!/usr/bin/env bash

set -euo pipefail

script_path="$(readlink -f -- "${BASH_SOURCE[0]}")"
script_dir="$(cd -- "$(dirname -- "$script_path")" && pwd -P)"
destination="$HOME/.local/bin/memo"
metadata_destination="$HOME/.local/lib/memo/metadata.sh"

if ! command -v cargo >/dev/null 2>&1; then
    printf 'memo installer: cargo is required to compile the CLI\n' >&2
    exit 1
fi

if [[ -L "$destination" ]]; then
    rm -- "$destination"
fi
mkdir -p -- "$(dirname -- "$destination")"
mkdir -p -- "$(dirname -- "$metadata_destination")"

printf '==> Compile memo CLI\n'
cargo install --path "$script_dir" --root "$HOME/.local" --force
install -m 0755 -- "$script_dir/metadata.sh" "$metadata_destination"
printf 'memo command: %s\n' "$destination"
printf 'metadata script: %s\n' "$metadata_destination"
