#!/usr/bin/env bash

set -euo pipefail

script_path="$(readlink -f -- "${BASH_SOURCE[0]}")"
script_dir="$(cd -- "$(dirname -- "$script_path")" && pwd -P)"
destination="$HOME/.local/bin/memo"

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
printf 'memo command: %s\n' "$destination"
