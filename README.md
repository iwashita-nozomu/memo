# memo

memo is a small Rust CLI for recording short, context-aware Markdown notes.
The command does not open an editor. Each note is immutable and stored in its
own content-addressed file, mirrors tagged entries, and can commit/push the
changed files in the background.

## Install

Rust/Cargo is the only build prerequisite:

    bash ./install.sh

The installer compiles the local source into ~/.local/bin/memo. The config is
managed by the integration that invokes this installer; create
~/.config/memo/config.toml and set the repository that should hold notes:

    repo = "~/til"
    auto_sync = true
    # metadata_script = "~/.config/memo/metadata.sh"
    # remote = "git@github.com:your-account/your-memo-repo.git"
    # environment = "gpu003-ubuntu2404"

XDG_CONFIG_HOME is honored by the CLI. MEMO_CONFIG can select one config file
for a test or a special environment.

The installer places the default metadata collector at
`~/.local/lib/memo/metadata.sh`. Set `metadata_script` to an executable script
of your choice to replace it. The script runs from the memo command's current
directory and must print YAML fields without the `---` front matter markers.
The following context is provided as environment variables:

    MEMO_TAGS       # comma-separated normalized tags
    MEMO_MESSAGE    # complete memo body
    MEMO_CWD        # canonical current directory
    MEMO_ENVIRONMENT # configured environment override, when configured

The script may emit any metadata fields, including fields specific to a user's
machine or workflow. Its output becomes the front matter of the immutable note.

## Entries and tags

Each invocation creates one immutable note at:

    <repo>/memo/inbox/<sha256-id>

The environment comes from environment in the config, then
MEMO_ENVIRONMENT, WSL_DISTRO_NAME, or the short hostname. The Markdown
metadata includes timestamp, environment, tags, working directory, and Git
context when the current directory is inside a Git worktree.

The file contents use Markdown with YAML front matter. The metadata and body
are separated, while the complete file remains self-contained:

    ---
    timestamp: "2026-08-22T10:30:00+09:00"
    environment: "gpu003"
    tags:
      - "todo"
    cwd: "/home/niwashita/workspace"
    git:
      root: "/home/niwashita/workspace"
      branch: "main"
      head: "abc1234"
    ---

    p(x_t+1|x_t) ...

The SHA-256 file name is calculated from the complete file contents. Notes
are not edited in place: an expanded revision is a new file with a new SHA.
This keeps every note independently readable and makes relationships stable
when a revision records its parent ID.

    memo "通常のinboxメモ"
    memo tag:todo "あとで片付ける作業"
    memo --tag reference "参照用メモ"

Tags are kept in each entry's `tags:` metadata. Tagged entries are also
mirrored into their tag directory using the same SHA-256 file name.

When a tag is supplied, its entry is also written to:

    <repo>/memo/<tag>/<sha256-id>

For example, `tag:todo` writes to both the inbox file and the `todo` mirror.
New tags automatically create their mirror directory and are registered as
one tag per line in the user-local `~/.config/memo/tags` file. The tag registry
is not stored in the memo repository or dotfiles.

## Synchronization

With auto_sync = true (the default), memo prints saved as soon as the entry and
its mirrors are written. Fetch, reset to upstream when there are no pending
memo changes, commit, and push then run in a background worker. Because each
note has a unique content-addressed path, normal writes do not collide across
devices. A failed worker writes an untracked
`<repo>/.memo-sync-error.log`; a later successful sync removes it.

Set auto_sync = false for local-only notes. MEMO_AUTO_SYNC=0 is a temporary
override.

## Development

    cargo test
    cargo run -- --help

The standalone public repository is intended to contain this directory's
Rust source, installer, example configuration, tests, and documentation.
