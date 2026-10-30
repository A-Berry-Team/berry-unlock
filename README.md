# BerryUnlock

BerryUnlock is a small command-line tool that raises the frame-rate limit in several anime-style games. It is built for **Wine / Proton launchers** on Linux and also works on Windows. (Wine and Proton are compatibility layers that run Windows programs on Linux.)

It is a ground-up rewrite of [KeqingUnlock](https://github.com/TwintailTeam/KeqingUnlock). Compared with the original, it has a modular code base, least-privilege process access, bounds-checked memory handling, atomic file edits and an optimised release profile.

**Who it is for:** Linux players who start these games through Wine, Proton or umu, Windows players who want the same tool, and contributors who want to extend it.

## Quick start

1. **Get `berry_unlock.exe`.** Download it from the project's GitHub Releases page (a release is published whenever a version tag such as `v0.1.0` is pushed), or [build it yourself](docs/BUILDING.md).
2. **Pick your game ID and timing** from the [supported games](#supported-games) table. Some games need BerryUnlock to run *before* the game starts, and others need it to run *alongside* it.
3. **Run it in the same Wine prefix as the game.** A *prefix* is the folder that holds one Wine installation of Windows. For example:

   ```sh
   berry_unlock.exe --no-dialog run hkrpg_global 120
   ```

   See [docs/PROTON.md](docs/PROTON.md) for ready-made Wine, Proton and umu commands.
4. **Wire it into your launcher.** Always pass `--no-dialog`. Treat a non-zero exit code as "the game will run at its normal FPS" and never block the game launch on it.

> **Note:** The default build has no console window, so `games` and `--help` show nothing in a terminal. Use the console build for those commands ([docs/BUILDING.md](docs/BUILDING.md#console-build-for-debugging)).

## Supported games

| Game | ID | Method |
|---|---|---|
| Genshin Impact (global) | `hk4e_global` | In-memory write to `GenshinImpact.exe` |
| Honkai: Star Rail (global) | `hkrpg_global` | Registry JSON |
| Honkai Impact 3rd (global) | `bh3_global` | Registry JSON |
| Wuthering Waves (global) | `wuwa_global` | SQLite database and INI file in the game directory |
| Zenless Zone Zero | `nap_global` | None (the game has a built-in "Unlimited" option) |

FPS caps, timing requirements and known limitations for each game are in [docs/GAME_SUPPORT.md](docs/GAME_SUPPORT.md).

## Usage

```text
berry_unlock [OPTIONS] games
berry_unlock [OPTIONS] run <game_id> <target_fps> [refresh_delay_ms] [init_delay_ms] [game_path]
```

The positional layout of `run` is identical to KeqingUnlock, so launcher integrations only need the binary name changed.

### Arguments of `run`

| Argument | Meaning |
|---|---|
| `game_id` | One of the IDs listed by `games` (see the table above) |
| `target_fps` | FPS to apply. Must be greater than 0 |
| `refresh_delay_ms` | Genshin Impact only. Re-apply the value every N ms while the game runs. `0` (default) applies it once |
| `init_delay_ms` | Genshin Impact only. Wait N ms after the game process appears before scanning. Default `0` |
| `game_path` | Wuthering Waves only. The game install directory. Pass an empty string when the game does not need it |

Limits on the delays are described in [docs/GAME_SUPPORT.md](docs/GAME_SUPPORT.md#summary).

### Options

| Option | Meaning |
|---|---|
| `--log <file>` | Append logs to a file (the file is truncated automatically once it grows above 1 MiB) |
| `-v`, `--verbose` | Turn on debug logging |
| `--no-dialog` | Never show message boxes (recommended for launchers) |
| `--wait-timeout <s>` | Seconds to wait for the game process. Default `300`; `0` means wait forever |
| `-h`, `--help` | Show help |
| `-V`, `--version` | Show the version |

### Examples

```sh
# Genshin: re-apply 144 FPS every 200 ms, scan 5 s after the process appears
berry_unlock.exe --no-dialog --log berry.log run hk4e_global 144 200 5000

# Star Rail / Honkai 3rd: run BEFORE launching the game
berry_unlock.exe run hkrpg_global 120

# Wuthering Waves: needs the install directory; the game must be closed
berry_unlock.exe run wuwa_global 120 0 0 "Z:\home\<user>\Games\WutheringWaves"
```

Exit codes are listed in [docs/GAME_SUPPORT.md](docs/GAME_SUPPORT.md#exit-codes).

For running under Proton or umu, prefix requirements and a launcher integration recipe, read [docs/PROTON.md](docs/PROTON.md).

## Building from source

From a Linux machine with Rust and mingw-w64 installed:

```sh
make test      # portable unit tests on the host
make build     # -> dist/berry_unlock.exe
make verify    # confirms only Windows system DLLs are imported
```

Package names per distribution, plain `cargo` commands and the console build are in [docs/BUILDING.md](docs/BUILDING.md).

## Contributing and releases

See [CONTRIBUTING.md](CONTRIBUTING.md). Pushing a tag such as `v0.1.0` makes GitHub Actions build, verify and publish `berry_unlock.exe` and a zip file as a GitHub Release.

## Documentation

| Document | What it covers |
|---|---|
| [docs/GAME_SUPPORT.md](docs/GAME_SUPPORT.md) | Per-game behaviour, known limitations and exit codes |
| [docs/PROTON.md](docs/PROTON.md) | Using BerryUnlock with Wine, Proton and umu |
| [docs/BUILDING.md](docs/BUILDING.md) | Toolchain and build commands |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Code design and differences from KeqingUnlock |
| [docs/SECURITY.md](docs/SECURITY.md) | Threat model, hardening and how to report vulnerabilities |
| [CONTRIBUTING.md](CONTRIBUTING.md) | How to contribute |

## Disclaimer

BerryUnlock changes game settings. For Genshin Impact it also writes to the memory of the running game process. Game publishers may treat this as a modification of their software. Use it at your own risk. The authors are not responsible for any action taken against your account.

## License

GPL-3.0-only. See [LICENSE](LICENSE) and [NOTICE](NOTICE). BerryUnlock is a derivative work of KeqingUnlock by TwintailTeam, so it must stay under the same licence.
