#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
if [[ -f "$script_dir/Cargo.toml" ]]; then
    repo_root="$script_dir"
    manifest="$repo_root/Cargo.toml"
else
    repo_root="$(cd -- "$script_dir/.." && pwd -P)"
    manifest="$repo_root/memo/Cargo.toml"
fi
rustup_home="${RUSTUP_HOME:-$HOME/.rustup}"
cargo_home="${CARGO_HOME:-$HOME/.cargo}"
rustup_toolchain="${RUSTUP_TOOLCHAIN:-stable-x86_64-unknown-linux-gnu}"

if ! command -v cargo >/dev/null 2>&1; then
    printf 'skip - cargo is unavailable; Rust tests will run during bootstrap\n'
    exit 0
fi

cargo test --manifest-path "$manifest"

work="$(mktemp -d "${TMPDIR:-/tmp}/memo-cli-test.XXXXXX")"
trap 'rm -rf -- "$work"' EXIT

git -C "$work" init --initial-branch=main >/dev/null
git -C "$work" config user.name 'memo test'
git -C "$work" config user.email 'memo-test@example.invalid'
printf '# memo test\n' > "$work/README.md"
git -C "$work" add README.md
git -C "$work" commit -m seed >/dev/null
mkdir -p "$work/home/.config/memo"
printf 'repo = "%s"\nenvironment = "test"\nauto_sync = false\n' "$work" > "$work/home/.config/memo/config.toml"

output="$(
    HOME="$work/home" \
    MEMO_CONFIG="$work/home/.config/memo/config.toml" \
    MEMO_AUTO_SYNC=0 \
    MEMO_ENVIRONMENT=test \
    RUSTUP_HOME="$rustup_home" \
    CARGO_HOME="$cargo_home" \
    RUSTUP_TOOLCHAIN="$rustup_toolchain" \
    cargo run --quiet --manifest-path "$manifest" -- --tag todo 'test memo'
)"
entry="$(printf '%s\n' "$output" | tail -n 1)"
entry="${entry#saved: }"
test -f "$entry"
mirror_path="${entry/\/inbox\//\/todo\/}"
test -f "$mirror_path"
grep -Fq 'tags: todo' "$entry"
grep -Fq 'test memo' "$entry"
printf 'ok - Rust memo writes inbox entries and tag mirrors\n'
