#!/usr/bin/env bash
# Bundles the release artefacts into dist/berry_unlock-<version>-x86_64-windows.zip
set -euo pipefail
cd "$(dirname "$0")/.."
[[ -f dist/berry_unlock.exe ]] || { echo "run scripts/build.sh first" >&2; exit 1; }
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n1)
OUT="dist/berry_unlock-${VERSION}-x86_64-windows.zip"
rm -f "$OUT"
zip -j "$OUT" dist/berry_unlock.exe dist/berry_unlock.exe.sha256 README.md LICENSE NOTICE docs/GAME_SUPPORT.md docs/PROTON.md
echo "Created $OUT"
