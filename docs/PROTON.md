# Using BerryUnlock with Wine / Proton

This guide shows how to run BerryUnlock inside the Wine environment that launches your game, with copy-ready commands for plain Wine, Proton and umu-launcher. It also gives a launcher integration recipe and a troubleshooting table.

`berry_unlock.exe` is a normal Windows program. For the in-memory method (Genshin Impact) and the registry method (Star Rail, Honkai Impact 3rd), it **must run inside the same Wine prefix as the game**. A *prefix* is the folder that holds one Wine installation of Windows. BerryUnlock talks to the game through that prefix's *wineserver* (the Wine background process that manages processes and the registry for the prefix).

## Rules of thumb

1. Use the **same prefix and the same runner** (the Proton or umu build) that launches the game.
2. For the registry games (`hkrpg_global`, `bh3_global`) and for `wuwa_global`, run BerryUnlock **first**, wait for it to exit, then start the game.
3. For Genshin Impact (`hk4e_global`), start BerryUnlock **together with** the game, either just before or just after it. BerryUnlock waits for the game process (300 s by default).
4. Always pass `--no-dialog` from launchers. Add `--log <file>` while troubleshooting.
5. Pass an empty string for `game_path` when the game does not need it.

## Commands

### Plain Wine

```sh
WINEPREFIX=~/.wine-genshin wine berry_unlock.exe --no-dialog run hk4e_global 144 200 5000 ""
```

### Proton (Steam runtime or standalone)

```sh
STEAM_COMPAT_DATA_PATH=~/Games/genshin/compat \
STEAM_COMPAT_CLIENT_INSTALL_PATH=~/.steam/steam \
~/.steam/steam/compatibilitytools.d/<proton>/proton run \
  ./berry_unlock.exe --no-dialog run hk4e_global 144 200 5000 ""
```

Replace `<proton>` with the name of your Proton folder.

### umu-launcher

```sh
WINEPREFIX=~/Games/genshin/pfx PROTONPATH=GE-Proton GAMEID=umu-0 \
umu-run ./berry_unlock.exe --no-dialog run hkrpg_global 120
```

### Paths

Windows code interprets paths such as `game_path`. Use a Wine path (`Z:\home\<user>\...`) or a Unix path that Wine can resolve.

## Launcher integration recipe

1. For registry games and Wuthering Waves, run `berry_unlock ...` and wait for exit code `0`.
2. Start the game through the same runner.
3. For Genshin Impact, start `berry_unlock` in the background with a refresh delay. It exits on its own when the game exits.

Treat a non-zero exit code as "the game will run at its normal FPS". Never block the game launch on it. Exit codes are listed in [GAME_SUPPORT.md](GAME_SUPPORT.md#exit-codes).

## Troubleshooting

| Symptom | Likely cause and fix |
|---|---|
| `OpenProcess failed (Win32 error 5)`, or a timeout while waiting for the game | BerryUnlock is not in the same prefix or wineserver as the game, or the game runs in a different container. Launch both with the same runner and prefix. |
| `Failed to locate FPS variable` | The game build changed its code (see [GAME_SUPPORT.md](GAME_SUPPORT.md#genshin-impact-hk4e_global)), or `init_delay_ms` is far too small. Try again with `-v --log berry.log`. |
| `registry key HKCU\... not found`, or no `GraphicsSettings` value | Launch the game once in that prefix, with the Custom graphics preset. |
| Wuthering Waves: file does not exist | The `game_path` is wrong, or the game was never launched. |
| The exe fails to start in a bare prefix | Run `make verify`. The exe must import only Windows system DLLs. Old Wine builds may lack APIs that Rust's standard library needs, so use a recent Wine or Proton. |
| Nothing is visible for `games` or `--help` | The default build has no console. Use the console build ([BUILDING.md](BUILDING.md#console-build-for-debugging)). |
