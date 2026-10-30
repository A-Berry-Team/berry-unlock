# Architecture

This page explains how BerryUnlock's code is organised and why. It is written for contributors. It covers the design principles, the Genshin Impact control flow and the differences from KeqingUnlock, the project this one rewrites.

## Design principles

1. **Portable core, thin platform shell.** Everything that can be pure is pure. It works on `&[u8]`, `&str` or the `MemoryReader` abstraction. The Windows layer only adapts operating-system calls to those abstractions. As a result, `cargo test --lib` exercises the scanner, the PE parser (PE is the Windows executable format), the FPS locator (against a fake process image), the JSON and INI patchers and the SQLite logic on a plain Linux host.
2. **Unsafe code is quarantined.** The crate sets `#![deny(unsafe_code)]`. Only `platform/` re-allows it, with one FFI call (a call into the operating system's C interface) per block. Each call sits behind an RAII wrapper (a type that releases its resource automatically when it goes out of scope): `Snapshot`, `RemoteProcess` and `RegistryKey`. Handles therefore cannot leak.
3. **Addresses in other processes are integers**, never Rust pointers. Relative offsets use wrapping arithmetic, and results are range-checked.
4. **Fail closed.** Any parse or lookup failure aborts that game's operation without writing anything, and the game simply starts with its normal FPS. Errors carry context and map to stable exit codes (see [GAME_SUPPORT.md](GAME_SUPPORT.md#exit-codes)).
5. **Few dependencies.** `memchr`, `serde_json`, `rusqlite` (with bundled SQLite) and `windows-sys` (raw Windows bindings). Compared with KeqingUnlock, these were removed: `clap`, `serde` derive, `winreg`, `configparser` and `windows`.

## Genshin Impact flow

```text
wait_for_module(GenshinImpact.exe, timeout)     200 ms poll, RAII handle (VM r/w/op + SYNCHRONIZE)
  -> wait_exit(init_delay)                      interruptible sleep
  -> locate_fps_variable() with 20 s retry      PE parse -> scan executable sections ->
                                                resolve each hit -> range + readability check
  -> write initial value
  -> loop { wait_exit(refresh) ; read ; write only if different }   exits with the game
```

In the first line, "VM r/w/op" stands for the process rights *virtual-memory read, write and operation*. `SYNCHRONIZE` lets the program wait for the process to exit.

## Differences from KeqingUnlock

| Area | KeqingUnlock | BerryUnlock |
|---|---|---|
| Process rights | `PROCESS_ALL_ACCESS` | VM read/write/operation, query-limited and synchronize |
| Waiting for the game | Infinite poll, no timeout | `--wait-timeout` (default 300 s) |
| Refresh loop | Blind write plus `sleep`; stops on the first failed write | Waits on the process handle, writes only on change, tolerates transient failures |
| Pattern scan | Naive matcher (could miss overlaps), 64 KiB chunks | Correct matcher, `memchr`, 1 MiB chunks with exact overlap |
| Candidate handling | First hit only | Every hit is tried, and the resolved address is range-checked |
| PE parsing | Struct casts on a raw buffer | Bounds-checked, honours `SizeOfOptionalHeader` |
| Registry | `winreg`, opened twice, UTF-8 rewrite | Direct bindings, opened once, NUL terminator preserved, key order preserved |
| INI (Wuthering Waves) | Re-serialised the whole file (lower-cased keys, comments lost) | In-place edit, atomic write, backup |
| SQLite (Wuthering Waves) | `Connection::open` (could create a database), `unwrap()` calls | Opens without CREATE, single transaction, backup, errors propagated |
| Errors | `unwrap` and `panic` paths, message boxes inline | One `Error` type, exit codes, optional dialogs, log file |
| CLI | `clap` derive | Hand-written parser, extra options, optional trailing arguments |
| Release profile | `opt-level = "s"`, 16 codegen units, no `panic = "abort"` | `opt-level = 3`, fat LTO, 1 codegen unit, `panic = "abort"`, static CRT (the C runtime library) |

For the safety measures that follow from these principles, see [SECURITY.md](SECURITY.md).
