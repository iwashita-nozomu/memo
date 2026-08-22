#!/usr/bin/env bash

set -euo pipefail

yaml_quote() {
    local value="$1"
    value="${value//\\/\\\\}"
    value="${value//\"/\\\"}"
    value="${value//$'\n'/\\n}"
    value="${value//$'\r'/\\r}"
    value="${value//$'\t'/\\t}"
    printf '"%s"' "$value"
}

environment="${MEMO_ENVIRONMENT:-}"
if [[ -z "$environment" && -n "${WSL_DISTRO_NAME:-}" ]]; then
    environment="wsl-$WSL_DISTRO_NAME"
fi
if [[ -z "$environment" ]]; then
    environment="$(hostname -s 2>/dev/null || hostname)"
fi
environment="$(printf '%s' "$environment" | LC_ALL=C sed 's/[^A-Za-z0-9._-]/_/g')"
environment="${environment:-unknown-environment}"

printf 'timestamp: '; yaml_quote "${MEMO_TIMESTAMP:-$(date +%Y-%m-%dT%H:%M:%S%:z)}"; printf '\n'
printf 'environment: '; yaml_quote "$environment"; printf '\n'
if [[ -z "${MEMO_TAGS:-}" ]]; then
    printf 'tags: []\n'
else
    printf 'tags:\n'
    IFS=',' read -r -a tags <<< "$MEMO_TAGS"
    for tag in "${tags[@]}"; do
        printf '  - '; yaml_quote "$tag"; printf '\n'
    done
fi
printf 'cwd: '; yaml_quote "${MEMO_CWD:-$PWD}"; printf '\n'

if command -v git >/dev/null 2>&1; then
    git_root="$(git rev-parse --show-toplevel 2>/dev/null || true)"
else
    git_root=""
fi
if [[ -n "$git_root" ]]; then
    git_branch="$(git branch --show-current 2>/dev/null || true)"
    git_head="$(git rev-parse --short HEAD 2>/dev/null || true)"
    [[ -n "$git_branch" ]] || git_branch=detached
    printf 'git:\n'
    printf '  root: '; yaml_quote "$git_root"; printf '\n'
    printf '  branch: '; yaml_quote "$git_branch"; printf '\n'
    printf '  head: '; yaml_quote "$git_head"; printf '\n'
else
    printf 'git: {}\n'
fi
