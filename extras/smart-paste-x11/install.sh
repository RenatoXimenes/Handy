#!/usr/bin/env bash
set -euo pipefail

source_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
target="$HOME/.local/bin/handy-smart-paste"

for command in xdotool xprop xsel timeout; do
  if ! command -v "$command" >/dev/null 2>&1; then
    printf 'Missing dependency: %s\n' "$command" >&2
    printf 'Ubuntu/Debian: sudo apt install xdotool x11-utils xsel coreutils\n' >&2
    exit 1
  fi
done

mkdir -p "$HOME/.local/bin"
install -m 0755 "$source_dir/handy-smart-paste" "$target"

printf 'Installed: %s\n' "$target"
printf 'In Handy, select paste method "External script" and choose that path.\n'
