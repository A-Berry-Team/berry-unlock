# Contributing to BerryUnlock

This guide explains how to prepare a change and what to include when reporting a problem. Contributions are welcome, from small fixes to support for new game builds.

## Making a code change

1. Fork the repository, create a branch and keep your changes focused.
2. Put platform-independent logic in `src/engine/` and cover it with unit tests.
3. Use `unsafe` only in `src/platform/`. Keep to one FFI call (a call into the operating system's C interface) per `unsafe` block, and add a `// SAFETY:` comment that explains why the call is sound. The design reasons are in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).
4. Before you open a pull request, run:

   ```sh
   cargo test --lib
   cargo check --target x86_64-pc-windows-gnu
   cargo fmt --all
   ```

   `make lint` runs the stricter checks (rustfmt in check mode and clippy with warnings denied, on the host and the Windows target). See [docs/BUILDING.md](docs/BUILDING.md) for the toolchain setup.
5. Contributions are licensed under GPL-3.0-only, like the rest of the project.

## Reporting a game that stopped working

When a game update breaks detection:

1. Open an issue using the bug report or game build validation template.
2. Include the game version.
3. Attach the output of a run with `-v --log <file>`.
4. Update the validation table in [docs/GAME_SUPPORT.md](docs/GAME_SUPPORT.md).

## Reporting a security problem

Do not open a public issue. Follow [docs/SECURITY.md](docs/SECURITY.md#reporting-vulnerabilities).
