# memo

memo is a small Rust CLI for recording short, context-aware Markdown notes.
The command does not open an editor. It appends each entry to one daily file
for the current environment and can commit/push that file in the background.

## Install

Rust/Cargo is the only build prerequisite:

    bash ./install.sh

The installer compiles the local source into ~/.local/bin/memo. The config is
managed by the integration that invokes this installer; create
~/.config/memo/config.toml and set the repository that should hold notes:

    repo = "~/til"
    auto_sync = true
    # remote = "git@github.com:your-account/your-memo-repo.git"
    # environment = "gpu003-ubuntu2404"

XDG_CONFIG_HOME is honored by the CLI. MEMO_CONFIG can select one config file
for a test or a special environment.

## Entries and tags

Entries are appended to:

    <repo>/memo/inbox/<environment>/YYYY-MM-DD.md

The environment comes from environment in the config, then
MEMO_ENVIRONMENT, WSL_DISTRO_NAME, or the short hostname. The Markdown
metadata includes timestamp, environment, tags, working directory, and Git
context when the current directory is inside a Git worktree.

    memo "通常のinboxメモ"
    memo tag:todo "あとで片付ける作業"
    memo --tag reference "参照用メモ"

Tags are kept in each entry's `tags:` metadata. No tag mirror files are
created, so a memo command only adds to the current environment/date file.

## Synchronization

With auto_sync = true (the default), memo prints saved as soon as the entry is
appended. Fetch, reset to upstream when there are no pending memo changes,
commit, and push then run in a background worker. Rebase conflicts in daily
memo files are merged as append-only content so entries from both devices are
kept. A failed worker writes an untracked `<repo>/.memo-sync-error.log`; a
later successful sync removes it.

Set auto_sync = false for local-only notes. MEMO_AUTO_SYNC=0 is a temporary
override.

## Development

    cargo test
    cargo run -- --help

The standalone public repository is intended to contain this directory's
Rust source, installer, example configuration, tests, and documentation.
