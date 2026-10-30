//! Wuthering Waves: patches `LocalStorage.db` (SQLite) and `GameUserSettings.ini`.
//! Must run while the game is NOT running (the game rewrites both on exit).
//! Fully portable: also works natively on Linux against a game directory.

use std::path::Path;
use std::time::Duration;

use rusqlite::{params, Connection, OpenFlags};

use super::wuwa_data::{menu_data_json, play_menu_info_json};
use crate::cli::RunArgs;
use crate::engine::{fsutil, ini_edit};
use crate::error::{Error, Result};
use crate::{log_info, log_warn};

const DB_REL: &str = "Client/Saved/LocalStorage/LocalStorage.db";
const INI_REL: &str = "Client/Saved/Config/WindowsNoEditor/GameUserSettings.ini";
const INI_SECTION: &str = "/Script/Engine.GameUserSettings";
const MAX_FPS: u32 = 120;
const TRIGGER_NAME: &str = "prevent_custom_frame_rate_update";

pub fn run(args: &RunArgs) -> Result<()> {
    if args.game_path.as_os_str().is_empty() {
        return Err(Error::Usage("wuwa_global requires <game_path> (the game install directory)".into()));
    }
    let db = args.game_path.join(DB_REL);
    let ini = args.game_path.join(INI_REL);
    fsutil::require_file(&db).map_err(hint_first_launch)?;
    fsutil::require_file(&ini).map_err(hint_first_launch)?;

    let fps = args.target_fps.min(MAX_FPS);
    for f in [&db, &ini] {
        match fsutil::backup_once(f) {
            Ok(Some(b)) => log_info!("backup created: {}", b.display()),
            Ok(None) => {}
            Err(e) => log_warn!("backup skipped: {e}"),
        }
    }

    patch_database(&db, fps)?;
    patch_ini(&ini, fps)?;
    log_info!("Wuthering Waves FPS unlocked to {fps}");
    Ok(())
}

fn hint_first_launch(e: Error) -> Error {
    match e {
        Error::NotFound(m) => Error::NotFound(format!("{m}. Launch the game once so it creates its settings files.")),
        other => other,
    }
}

pub fn trigger_sql(fps: u32) -> String {
    format!(
        "CREATE TRIGGER {TRIGGER_NAME} AFTER UPDATE OF value ON LocalStorage \
         WHEN NEW.key = 'CustomFrameRate' \
         BEGIN UPDATE LocalStorage SET value = {fps} WHERE key = 'CustomFrameRate'; END;"
    )
}

/// All-or-nothing: everything happens in one transaction.
pub fn patch_database(path: &Path, fps: u32) -> Result<()> {
    // READ_WRITE without CREATE: a wrong path must fail, not create an empty DB.
    let mut conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
    conn.busy_timeout(Duration::from_secs(10))?;
    let tx = conn.transaction()?;

    let has_table: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'LocalStorage')",
        [],
        |r| r.get(0),
    )?;
    if !has_table {
        return Err(Error::InvalidData("LocalStorage table not found; is this the right database?".into()));
    }

    tx.execute_batch(&format!("DROP TRIGGER IF EXISTS {TRIGGER_NAME};"))?;

    let updated = tx.execute(
        "UPDATE LocalStorage SET value = ?1 WHERE key = 'CustomFrameRate'",
        params![fps.to_string()],
    )?;
    if updated == 0 {
        if let Err(e) = tx.execute(
            "INSERT INTO LocalStorage (key, value) VALUES ('CustomFrameRate', ?1)",
            params![fps.to_string()],
        ) {
            log_warn!("CustomFrameRate row missing and could not be inserted: {e}");
        }
    }

    tx.execute("DELETE FROM LocalStorage WHERE key IN ('MenuData', 'PlayMenuInfo')", [])?;
    tx.execute("INSERT INTO LocalStorage (key, value) VALUES (?1, ?2)", params!["MenuData", menu_data_json()])?;
    tx.execute("INSERT INTO LocalStorage (key, value) VALUES (?1, ?2)", params!["PlayMenuInfo", play_menu_info_json()])?;

    // Re-asserts the value whenever the game tries to change it.
    tx.execute_batch(&trigger_sql(fps))?;
    tx.commit()?;
    Ok(())
}

pub fn patch_ini(path: &Path, fps: u32) -> Result<()> {
    let original = std::fs::read_to_string(path).map_err(|e| Error::io(format!("reading {}", path.display()), e))?;
    let updated = ini_edit::set_key(&original, INI_SECTION, "FramePace", &fps.to_string());
    fsutil::atomic_write(path, updated.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn temp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("berry_wuwa_{}_{}", std::process::id(), name))
    }

    fn make_db(path: &Path) {
        let c = Connection::open(path).unwrap();
        c.execute_batch(
            "CREATE TABLE LocalStorage (key TEXT PRIMARY KEY, value TEXT);
             INSERT INTO LocalStorage VALUES ('CustomFrameRate', '60');
             INSERT INTO LocalStorage VALUES ('MenuData', 'old');
             INSERT INTO LocalStorage VALUES ('Unrelated', 'keep');",
        )
        .unwrap();
    }

    fn get(c: &Connection, k: &str) -> Option<String> {
        c.query_row("SELECT value FROM LocalStorage WHERE key = ?1", params![k], |r| r.get(0)).ok()
    }

    #[test]
    fn database_patch_sets_rows_and_trigger() {
        let p = temp("a.db");
        let _ = fs::remove_file(&p);
        make_db(&p);
        patch_database(&p, 120).unwrap();

        let c = Connection::open(&p).unwrap();
        assert_eq!(get(&c, "CustomFrameRate").as_deref(), Some("120"));
        assert_eq!(get(&c, "Unrelated").as_deref(), Some("keep"));
        assert!(get(&c, "MenuData").unwrap().contains("___Map___"));
        assert!(get(&c, "PlayMenuInfo").unwrap().starts_with('{'));

        // The trigger must undo a later change by the game.
        c.execute("UPDATE LocalStorage SET value = '30' WHERE key = 'CustomFrameRate'", []).unwrap();
        assert_eq!(get(&c, "CustomFrameRate").as_deref(), Some("120"));
        drop(c);

        // Idempotent.
        patch_database(&p, 90).unwrap();
        let c = Connection::open(&p).unwrap();
        assert_eq!(get(&c, "CustomFrameRate").as_deref(), Some("90"));
        drop(c);
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn missing_database_is_an_error_and_not_created() {
        let p = temp("missing.db");
        let _ = fs::remove_file(&p);
        assert!(patch_database(&p, 60).is_err());
        assert!(!p.exists());
    }

    #[test]
    fn wrong_database_is_rejected() {
        let p = temp("wrong.db");
        let _ = fs::remove_file(&p);
        Connection::open(&p).unwrap().execute_batch("CREATE TABLE other (a);").unwrap();
        assert!(matches!(patch_database(&p, 60), Err(Error::InvalidData(_))));
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn ini_patch_writes_frame_pace() {
        let p = temp("g.ini");
        fs::write(&p, "[/Script/Engine.GameUserSettings]\r\nFramePace=30\r\n").unwrap();
        patch_ini(&p, 120).unwrap();
        assert_eq!(fs::read_to_string(&p).unwrap(), "[/Script/Engine.GameUserSettings]\r\nFramePace=120\r\n");
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn run_requires_game_path() {
        let args = RunArgs {
            game: crate::game_id::GameId::WutheringWavesGlobal,
            target_fps: 60,
            refresh_delay_ms: 0,
            init_delay_ms: 0,
            game_path: PathBuf::new(),
        };
        assert!(matches!(run(&args), Err(Error::Usage(_))));
    }
}
