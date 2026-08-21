# memo

memo is a small Rust CLI for recording short, context-aware Markdown notes.
The command does not open an editor. It writes an entry, optionally creates tag
mirrors, and can commit/push only the files it just created.

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

An entry is stored under:

    <repo>/memo/inbox/<environment>/YYYY-MM-DD/<timestamp>_<random>.md

The environment comes from environment in the config, then
MEMO_ENVIRONMENT, WSL_DISTRO_NAME, or the short hostname. The Markdown
metadata includes timestamp, environment, tags, working directory, and Git
context when the current directory is inside a Git worktree.

    memo "通常のinboxメモ"
    memo tag:todo "あとで片付ける作業"
    memo --tag reference "参照用メモ"

Every tag except inbox is mirrored at
<repo>/memo/<tag>/<environment>/YYYY-MM-DD/. The inbox entry is the source;
mirrors are generated when the entry is created.

## Synchronization

With auto_sync = true (the default), only the new entry and its mirrors are
staged and committed. If the checkout has an upstream, the commit is pushed
and the command prints commit/push progress. A push race is retried with
fetch/rebase only when the worktree was clean and had no existing unpushed
commits. Existing changes are never staged, rebased, or pushed implicitly.

Set auto_sync = false for local-only notes. MEMO_AUTO_SYNC=0 is a temporary
override.

## Development

    cargo test
    cargo run -- --help

The standalone public repository is intended to contain this directory's
Rust source, installer, example configuration, tests, and documentation.
