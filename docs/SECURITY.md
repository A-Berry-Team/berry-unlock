# Security model and hardening

This page describes what BerryUnlock can access, how it limits its own power, what risks remain and how to report a vulnerability. It is written for users who want to know what the tool does on their system, and for contributors who must keep it safe.

## What BerryUnlock can touch

| Resource | Scope |
|---|---|
| Process memory | Only the process named `GenshinImpact.exe` in the current Wine prefix. It makes one 4-byte write at a validated address |
| Registry | One fixed `HKCU` subkey per game, and one value in it |
| Files | `LocalStorage.db` and `GameUserSettings.ini` under the `game_path` given on the command line (plus their `*.berry.bak` and `*.berry.tmp` siblings), and the optional `--log` file |
| Network | None. There is no networking code and there are no network dependencies |

BerryUnlock runs with the privileges of the user who starts it and never elevates them.

## Hardening measures

- **Least-privilege process handle.** `OpenProcess` requests only virtual-memory read, write and operation, `PROCESS_QUERY_LIMITED_INFORMATION` and `SYNCHRONIZE`. It does not request `PROCESS_ALL_ACCESS`.
- **No unchecked pointers.** Remote addresses are `usize` values. Header parsing is bounds-checked. The resolved write target must lie inside the game module image and be readable, so a false-positive signature cannot cause an arbitrary write.
- **Unsafe code isolation.** `#![deny(unsafe_code)]` applies crate-wide. All `unsafe` code lives in `src/platform/`, and each block carries a comment with its safety argument.
- **Strict input handling.** Game IDs are fixed, numbers are validated and delays are clamped. JSON, INI and SQL are built from constants. Only an integer FPS value is interpolated into SQL, and all other values are bound parameters.
- **Safe file handling.** Files are replaced atomically (temporary file, `fsync`, rename). Backups are made once. SQLite opens without `CREATE`, so a wrong path never creates a database. Changes run in a single transaction that rolls back on any error.
- **Bounded behaviour.** There is a wait timeout, a retry window, a failure counter and a log size cap.
- **No panics on input.** Errors are values. In release builds, `panic = "abort"` plus `overflow-checks = true` make internal bugs stop the process instead of continuing with corrupt state.
- **Self-contained binary.** The C runtime, libgcc and winpthread are linked statically. `scripts/verify.sh` rejects any build that imports non-system DLLs. Dependencies are few and can be pinned with `Cargo.lock` and `--locked` (see [BUILDING.md](BUILDING.md#cargolock-and-reproducible-builds)).

## Residual risks and non-goals

- **Process identification by name.** BerryUnlock finds the target process by its executable name. Another process called `GenshinImpact.exe` in the same prefix would be treated as the game. Checking the image path is a candidate future improvement.
- **Terms of service and anti-cheat.** Writing into another process's memory may violate a game's terms of service or be flagged by anti-cheat software. This risk comes with the Genshin Impact method.
- **Concurrent edits.** Registry and file edits assume the game is not rewriting them at the same moment.
- **No evasion.** The tool does not hide itself or try to evade any protection, and it will not.

## Reporting vulnerabilities

Please do not open a public issue. Open a private security advisory in the repository that hosts this project, or contact the maintainers directly.
