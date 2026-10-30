#!/usr/bin/env bash
# Cross-compiles dist/berry_unlock.exe (x86_64-pc-windows-gnu, release) from Linux.
# Usage: scripts/build.sh [--console] [extra cargo args...]
set -euo pipefail
cd "$(dirname "$0")/.."

TARGET="x86_64-pc-windows-gnu"
FEATURES=()
if [[ "${1:-}" == "--console" ]]; then
    FEATURES=(--features console)
    shift
fi

need() { command -v "$1" >/dev/null 2>&1 || { echo "error: '$1' not found in PATH (see docs/BUILDING.md)" >&2; exit 1; }; }
need cargo
need rustup
need x86_64-w64-mingw32-gcc

rustup target add "$TARGET" >/dev/null

LOCK=()
[[ -f Cargo.lock ]] && LOCK=(--locked)

cargo build --release --target "$TARGET" ${LOCK[@]+"${LOCK[@]}"} ${FEATURES[@]+"${FEATURES[@]}"} "$@"

mkdir -p dist
cp "target/$TARGET/release/berry_unlock.exe" dist/berry_unlock.exe
( cd dist && sha256sum berry_unlock.exe > berry_unlock.exe.sha256 )

echo
echo "Built: dist/berry_unlock.exe ($(du -h dist/berry_unlock.exe | cut -f1))"
cat dist/berry_unlock.exe.sha256
echo "Next: scripts/verify.sh"
