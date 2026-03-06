#!/usr/bin/env bash
# Syncs telltale source from WSL to Windows filesystem.
# Usage: ./sync-to-win.sh [src] [dst]
#   Defaults: SRC = current directory, DST = /mnt/c/Users/$USER/telltale

SRC="${1:-$(pwd)}"
DST="${2:-/mnt/c/Users/$USER/telltale}"

rsync -av --delete \
  --exclude target/ \
  --exclude target2/ \
  --exclude .git/ \
  --exclude node_modules/ \
  "$SRC/" "$DST/"

echo "synced to $DST"
