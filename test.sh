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
printf 'repo = "%s"\nenvironment = "test"\nmetadata_script = "%s"\nauto_sync = false\n' \
    "$work" "$repo_root/metadata.sh" > "$work/home/.config/memo/config.toml"

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
todo_mirror_dir="$work/memo/todo"
test -d "$todo_mirror_dir"
todo_mirror="$todo_mirror_dir/$(basename "$entry")"
test -f "$todo_mirror"
test "$(basename "$entry")" = "$(sha256sum "$entry" | awk '{print $1}')"
grep -Fq -- $'---\ntimestamp:' "$entry"
grep -Fq 'environment: "test"' "$entry"
grep -Fq '  - "todo"' "$entry"
grep -Fq 'test memo' "$entry"
grep -Fq 'test memo' "$todo_mirror"
grep -Fxq 'todo' "$work/home/.config/memo/tags"

second_output="$(
    HOME="$work/home" \
    MEMO_CONFIG="$work/home/.config/memo/config.toml" \
    MEMO_AUTO_SYNC=0 \
    MEMO_ENVIRONMENT=test \
    RUSTUP_HOME="$rustup_home" \
    CARGO_HOME="$cargo_home" \
    RUSTUP_TOOLCHAIN="$rustup_toolchain" \
    cargo run --quiet --manifest-path "$manifest" -- --tag reference 'second memo'
)"
second_entry="$(printf '%s\n' "$second_output" | tail -n 1)"
second_entry="${second_entry#saved: }"
test -f "$second_entry"
test "$entry" != "$second_entry"
test "$(basename "$second_entry")" = "$(sha256sum "$second_entry" | awk '{print $1}')"
reference_mirror_dir="$work/memo/reference"
reference_mirror="$reference_mirror_dir/$(basename "$second_entry")"
test -f "$reference_mirror"
grep -Fq '  - "reference"' "$second_entry"
! grep -Fq 'second memo' "$entry"
grep -Fq 'second memo' "$reference_mirror"
grep -Fxq 'reference' "$work/home/.config/memo/tags"

custom_script="$work/custom-metadata.sh"
printf '%s\n' \
    '#!/usr/bin/env bash' \
    'printf '\''custom: "yes"\nmessage_length: "%s"\n'\'' "${#MEMO_MESSAGE}"' \
    > "$custom_script"
chmod +x "$custom_script"
custom_config="$work/home/.config/memo/custom.toml"
printf 'repo = "%s"\nmetadata_script = "%s"\nauto_sync = false\n' \
    "$work" "$custom_script" > "$custom_config"
custom_output="$(
    HOME="$work/home" \
    MEMO_CONFIG="$custom_config" \
    MEMO_AUTO_SYNC=0 \
    RUSTUP_HOME="$rustup_home" \
    CARGO_HOME="$cargo_home" \
    RUSTUP_TOOLCHAIN="$rustup_toolchain" \
    cargo run --quiet --manifest-path "$manifest" -- 'custom metadata'
)"
custom_entry="${custom_output#saved: }"
test -f "$custom_entry"
grep -Fq 'custom: "yes"' "$custom_entry"
grep -Fq 'message_length: "15"' "$custom_entry"
printf 'ok - Rust memo writes immutable SHA files, mirrors tags, and accepts custom metadata scripts\n'
