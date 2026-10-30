# Building

This guide explains how to build `berry_unlock.exe` from source, check the result and run the tests. It is written for contributors and for anyone who wants to build their own binary.

The target is `x86_64-pc-windows-gnu`. You cross-compile (build for Windows on another operating system) from Linux. The output is `berry_unlock.exe`.

## Prerequisites

| Tool | Notes |
|---|---|
| Rust 1.82 or newer, via `rustup` | `rust-toolchain.toml` pins the `stable` channel and adds the Windows target |
| mingw-w64 (x86_64) | Provides the linker and C compiler. SQLite is compiled from its bundled C source |
| `zip`, `file`, `binutils-mingw-w64` | Needed only by `package.sh` and `verify.sh` |

Install mingw-w64:

```sh
# Debian / Ubuntu
sudo apt install gcc-mingw-w64-x86-64 binutils-mingw-w64-x86-64 zip
# Fedora
sudo dnf install mingw64-gcc mingw64-binutils zip
# Arch
sudo pacman -S mingw-w64-gcc zip
```

On Debian and Ubuntu, the default `x86_64-w64-mingw32-gcc` may be the *posix* thread-model variant. The static link flags in `.cargo/config.toml` already bundle `libwinpthread`, and `make verify` fails the build if any non-system DLL is still imported.

## Build in one go (recommended)

```sh
make test       # portable unit tests on the host
make build      # release build -> dist/berry_unlock.exe (+ .sha256)
make verify     # DLL import check
make package    # dist/berry_unlock-<version>-x86_64-windows.zip
```

The package zip contains `berry_unlock.exe`, its `.sha256` file, `README.md`, `LICENSE`, `NOTICE`, `docs/GAME_SUPPORT.md` and `docs/PROTON.md`.

## Plain cargo equivalents

```sh
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
# binary: target/x86_64-pc-windows-gnu/release/berry_unlock.exe
```

`.cargo/config.toml` defines two aliases: `cargo build-win` and `cargo check-win`.

### Console build (for debugging)

The default binary uses the GUI subsystem, so Wine never opens a console window. To see the output of `games`, `--help` and live logs in a terminal, build the console variant:

```sh
./scripts/build.sh --console
# or: cargo build --release --target x86_64-pc-windows-gnu --features console
```

`make build-console` does the same.

## `Cargo.lock` and reproducible builds

`Cargo.lock` records the exact dependency versions. If it is missing, generate it (this needs network access) and **commit it**:

```sh
cargo generate-lockfile
```

When `Cargo.lock` exists, `scripts/build.sh` passes `--locked`, so every build uses the same dependency versions.

## Release profile

These settings live in `Cargo.toml`.

| Setting | Value | Why |
|---|---|---|
| `opt-level` | `3` | Speed (the original used `s`) |
| `lto` | `fat` | Whole-program inlining (LTO means link-time optimisation) |
| `codegen-units` | `1` | Best optimisation |
| `panic` | `abort` | Smaller binary, no unwinding |
| `strip` | `symbols` | Smaller binary |
| `overflow-checks` | `true` | Integer bugs stop the program instead of corrupting data |
| `debug`, `debug-assertions`, `incremental` | off | Not needed in release builds |

The C runtime, libgcc and winpthread are linked statically through `.cargo/config.toml`. No `-C target-cpu` flag is set on purpose, so the binary runs on any x86-64 CPU.

## Tests and lint

```sh
cargo test --lib        # host tests: CLI, PE parser, scanner, locator, JSON/INI/SQLite patchers
make check-win          # type-checks the Windows-only layer
make lint               # rustfmt + clippy (host and Windows target)
```

## Troubleshooting

| Message | What to do |
|---|---|
| `linker x86_64-w64-mingw32-gcc not found` | Install mingw-w64. |
| `verify.sh` reports `libwinpthread-1.dll` | Your mingw-w64 lacks the static `libwinpthread.a`. Install the `-posix` variant package, or the `mingw-w64-winpthreads` static libraries. |
| `error: package requires rustc 1.82` | Run `rustup update stable`. |
