#!/usr/bin/env bash
# Checks that dist/berry_unlock.exe only imports Windows system DLLs, i.e. it is
# self-contained and will start in a bare Wine/Proton prefix.
set -euo pipefail
cd "$(dirname "$0")/.."

EXE="${1:-dist/berry_unlock.exe}"
[[ -f "$EXE" ]] || { echo "error: $EXE not found (run scripts/build.sh first)" >&2; exit 1; }
command -v x86_64-w64-mingw32-objdump >/dev/null || { echo "error: x86_64-w64-mingw32-objdump not found" >&2; exit 1; }

echo "== file type =="
file "$EXE" || true

echo "== imported DLLs =="
DLLS=$(x86_64-w64-mingw32-objdump -p "$EXE" | awk '/DLL Name:/ {print tolower($3)}' | sort -u)
echo "$DLLS"

BAD=$(echo "$DLLS" | grep -E '^(libgcc|libwinpthread|libstdc\+\+|libssp|msvcp|vcruntime|ucrtbase)' || true)
if [[ -n "$BAD" ]]; then
    echo
    echo "FAIL: the binary depends on runtime DLLs that a bare prefix does not provide:" >&2
    echo "$BAD" >&2
    echo "Check .cargo/config.toml (static linking flags) and your mingw-w64 install." >&2
    exit 1
fi
echo
echo "OK: only system DLLs are imported."
echo "== sha256 =="
sha256sum "$EXE"
