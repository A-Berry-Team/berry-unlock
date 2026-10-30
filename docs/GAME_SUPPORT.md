# Game support, behaviour and known limitations

This page describes how BerryUnlock treats each supported game: when to run it, which arguments matter, what it changes and what can go wrong. It also lists the exit codes. It is written for people who use BerryUnlock. For command-line syntax, see the [README](../README.md#usage).

## Summary

| Game | ID | Method | Run when | `game_path` | Delays | FPS cap |
|---|---|---|---|---|---|---|
| Genshin Impact | `hk4e_global` | In-memory write to `GenshinImpact.exe` | Any time | No | Yes | None* |
| Honkai: Star Rail | `hkrpg_global` | Registry JSON | Before launch | No | No | 120 |
| Honkai Impact 3rd | `bh3_global` | Registry JSON | Before launch | No | No | None* |
| Wuthering Waves | `wuwa_global` | SQLite database and INI file in the game directory | Before launch, game closed | Yes | No | 120 |
| Zenless Zone Zero | `nap_global` | None | Any time | No | No | Built-in Unlimited |

\* Genshin Impact writes `60` on the first write when `target_fps >= 500`. Honkai Impact 3rd always maps such values to `60`.

**About the delays.** `refresh_delay_ms` and `init_delay_ms` apply only to Genshin Impact.

- A non-zero `refresh_delay_ms` is clamped to the range 10–60000 ms. `0` means "apply once".
- `init_delay_ms` is capped at 600000 ms.

## Genshin Impact (`hk4e_global`)

**What it does**

- Waits for `GenshinImpact.exe`, finds the FPS variable through a code signature (a fixed byte pattern in the game's code) and writes the requested value.
- Retries detection for up to 20 seconds after the initialization delay. The game's code is only readable once it has unpacked itself, and Proton start-up times vary.
- With `refresh_delay_ms > 0`, re-applies the FPS value until the game exits.
- Stops when the game closes, or after 50 consecutive memory-access failures.

**Known limitations**

- Game updates may break the signature in `src/engine/fps_locator.rs`. A failed detection is safe: no memory is written.
- Only the global client is supported. `YuanShen.exe` is not.
- BerryUnlock must run in the same Wine prefix and wineserver (the Wine background process that manages a prefix) as the game. See [PROTON.md](PROTON.md).
- Use `--wait-timeout` if you start BerryUnlock well before the game.
- The in-game FPS menu may not show the real value.
- Modifying process memory is the most intrusive method BerryUnlock uses. See the [disclaimer in the README](../README.md#disclaimer).

## Honkai: Star Rail (`hkrpg_global`)

**What it does**

- Patches the `FPS` field in the registry key `HKCU\Software\Cognosphere\Star Rail`, in the value `GraphicsSettings_Model_h<hash>`.
- Caps FPS at **120**.
- Preserves JSON formatting details, including key order and a trailing NUL character.

**Known limitations**

- Launch the game once with the **Custom** graphics preset first, so the value exists.
- Run BerryUnlock before launching the game. Changing graphics settings in game may overwrite the patched value.
- Under Wine, use the game's own prefix.

## Honkai Impact 3rd (`bh3_global`)

**What it does**

- Patches the existing `TargetFrameRateForInLevel` and `TargetFrameRateForOthers` fields in `PersonalGraphicsSettingV2_h<hash>`, under `HKCU\Software\miHoYo\Honkai Impact 3rd`.
- Maps `target_fps >= 500` to `60`.

**Known limitations**

The limitations are the same as for Star Rail:

- The registry value must already exist.
- BerryUnlock must run before the game starts.
- Changing settings in game may revert the patch.

## Wuthering Waves (`wuwa_global`)

**What it does**

- Requires `<game_path>` to point to the directory that contains `Client/`.
- Patches two files:
  - `Client/Saved/LocalStorage/LocalStorage.db`
  - `Client/Saved/Config/WindowsNoEditor/GameUserSettings.ini`
- In the database, sets `CustomFrameRate`, replaces `MenuData` and `PlayMenuInfo`, and installs the `prevent_custom_frame_rate_update` trigger. In the INI file, updates `FramePace`.
- Caps FPS at **120**.
- Creates a one-time backup next to each file, named `*.berry.bak`.

**Known limitations**

- **It resets the graphics menu presets.** To get your previous choices back, restore the original database from its `.berry.bak` file.
- The game must have run at least once, and it must be **closed** while BerryUnlock patches. If the database is locked, BerryUnlock waits up to 10 seconds.
- The trigger handles `UPDATE` statements only. It does not protect against a row being replaced through `DELETE` plus `INSERT`.
- Because it edits plain SQLite and INI files, it also works natively on Linux. The intended use is still inside the game's prefix.

## Zenless Zone Zero (`nap_global`)

The game already has an **Unlimited** FPS option. BerryUnlock accepts the ID and exits successfully without changing anything.

## Not supported

BerryUnlock supports only the games listed on this page. See also the Genshin Impact note about `YuanShen.exe` above.

## Exit codes

| Code | Meaning |
|---|---|
| `0` | Success, or nothing to do |
| `1` | Runtime error (I/O, Win32, database or JSON) |
| `2` | Invalid usage |
| `3` | File, registry value or process not found, or a timeout |
| `4` | Memory signature not found |
| `5` | The game exited before BerryUnlock finished (no dialog is shown) |
