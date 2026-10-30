//! Hand-written argument parser (no `clap`: smaller binary, faster build, no
//! proc-macro supply chain). The positional layout of `run` is identical to
//! KeqingUnlock so existing launcher integrations keep working.

use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use crate::error::{Error, Result};
use crate::game_id::GameId;

pub const DEFAULT_WAIT_TIMEOUT_SECS: u64 = 300;
pub const MIN_REFRESH_MS: u64 = 10;
pub const MAX_REFRESH_MS: u64 = 60_000;
pub const MAX_INIT_DELAY_MS: u64 = 600_000;

pub const HELP: &str = "\
BerryUnlock - FPS unlocker for anime-style games (Wine/Proton friendly)

USAGE:
    berry_unlock [OPTIONS] games
    berry_unlock [OPTIONS] run <game_id> <target_fps> [refresh_delay_ms] [init_delay_ms] [game_path]

ARGUMENTS:
    game_id            see `games`
    target_fps         FPS to apply (must be > 0)
    refresh_delay_ms   Genshin only: re-apply every N ms while the game runs (0 = once, default 0)
    init_delay_ms      Genshin only: wait N ms after the process appears before scanning (default 0)
    game_path          Wuthering Waves only: game install directory

OPTIONS:
        --log <file>          append log output to <file>
    -v, --verbose             enable debug logging
        --no-dialog           never show message boxes (headless / launcher use)
        --wait-timeout <s>    seconds to wait for the game process (default 300, 0 = forever)
    -h, --help                show this help
    -V, --version             show version
";

#[derive(Debug, Clone)]
pub struct Options {
    pub log_file: Option<PathBuf>,
    pub verbose: bool,
    pub dialogs: bool,
    pub wait_timeout: Option<Duration>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            log_file: None,
            verbose: false,
            dialogs: true,
            wait_timeout: Some(Duration::from_secs(DEFAULT_WAIT_TIMEOUT_SECS)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RunArgs {
    pub game: GameId,
    pub target_fps: u32,
    pub refresh_delay_ms: u64,
    pub init_delay_ms: u64,
    pub game_path: PathBuf,
}

#[derive(Debug)]
pub enum Command {
    Games,
    Help,
    Version,
    Run(RunArgs),
}

#[derive(Debug)]
pub struct Cli {
    pub options: Options,
    pub command: Command,
}

fn usage(msg: impl Into<String>) -> Error {
    Error::Usage(format!("{}\nTry `berry_unlock --help`.", msg.into()))
}

fn parse_num<T: FromStr>(name: &str, raw: &str) -> Result<T> {
    raw.trim()
        .parse::<T>()
        .map_err(|_| usage(format!("invalid value for {name}: '{raw}'")))
}

pub fn parse<I, S>(args: I) -> Result<Cli>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut it = args.into_iter().map(|s| -> String { s.into() });
    let mut options = Options::default();
    let mut positional: Vec<String> = Vec::new();
    let (mut help, mut version, mut literal) = (false, false, false);

    while let Some(arg) = it.next() {
        if literal || !arg.starts_with('-') || arg == "-" {
            positional.push(arg);
            continue;
        }
        match arg.as_str() {
            "--" => literal = true,
            "-h" | "--help" => help = true,
            "-V" | "--version" => version = true,
            "-v" | "--verbose" => options.verbose = true,
            "--no-dialog" => options.dialogs = false,
            "--log" => {
                let v = it.next().ok_or_else(|| usage("--log requires a file path"))?;
                options.log_file = Some(PathBuf::from(v));
            }
            "--wait-timeout" => {
                let v = it.next().ok_or_else(|| usage("--wait-timeout requires a number of seconds"))?;
                let secs: u64 = parse_num("--wait-timeout", &v)?;
                options.wait_timeout = if secs == 0 { None } else { Some(Duration::from_secs(secs)) };
            }
            other => return Err(usage(format!("unknown option '{other}'"))),
        }
    }

    if help {
        return Ok(Cli { options, command: Command::Help });
    }
    if version {
        return Ok(Cli { options, command: Command::Version });
    }

    let mut pos = positional.into_iter();
    let command = match pos.next().as_deref() {
        None => return Err(usage("no subcommand specified")),
        Some("games") => Command::Games,
        Some("help") => Command::Help,
        Some("version") => Command::Version,
        Some("run") => {
            let game_raw = pos.next().ok_or_else(|| usage("run: missing <game_id>"))?;
            let game = GameId::parse(&game_raw)
                .ok_or_else(|| usage(format!("run: unknown game id '{game_raw}' (see `games`)")))?;
            let fps_raw = pos.next().ok_or_else(|| usage("run: missing <target_fps>"))?;
            let target_fps: u32 = parse_num("target_fps", &fps_raw)?;
            if target_fps == 0 {
                return Err(usage("run: target_fps must be greater than 0"));
            }
            let refresh_delay_ms = match pos.next() {
                Some(s) => parse_num::<u64>("refresh_delay_ms", &s)?,
                None => 0,
            };
            let init_delay_ms = match pos.next() {
                Some(s) => parse_num::<u64>("init_delay_ms", &s)?,
                None => 0,
            };
            let game_path = PathBuf::from(pos.next().unwrap_or_default());
            if pos.next().is_some() {
                return Err(usage("run: too many arguments"));
            }
            Command::Run(RunArgs {
                game,
                target_fps,
                refresh_delay_ms: if refresh_delay_ms == 0 {
                    0
                } else {
                    refresh_delay_ms.clamp(MIN_REFRESH_MS, MAX_REFRESH_MS)
                },
                init_delay_ms: init_delay_ms.min(MAX_INIT_DELAY_MS),
                game_path,
            })
        }
        Some(other) => return Err(usage(format!("unknown subcommand '{other}'"))),
    };
    Ok(Cli { options, command })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(args: &[&str]) -> Result<Cli> {
        parse(args.iter().map(|s| s.to_string()))
    }

    #[test]
    fn parses_full_run_command() {
        let cli = p(&["run", "hk4e_global", "144", "200", "5000", "/games/x"]).unwrap();
        match cli.command {
            Command::Run(r) => {
                assert_eq!(r.game, GameId::GenshinGlobal);
                assert_eq!(r.target_fps, 144);
                assert_eq!(r.refresh_delay_ms, 200);
                assert_eq!(r.init_delay_ms, 5000);
                assert_eq!(r.game_path, PathBuf::from("/games/x"));
            }
            _ => panic!("expected run"),
        }
    }

    #[test]
    fn optional_trailing_arguments_default() {
        let cli = p(&["run", "hkrpg_global", "120"]).unwrap();
        match cli.command {
            Command::Run(r) => {
                assert_eq!((r.refresh_delay_ms, r.init_delay_ms), (0, 0));
                assert!(r.game_path.as_os_str().is_empty());
            }
            _ => panic!(),
        }
    }

    #[test]
    fn empty_game_path_is_accepted() {
        assert!(p(&["run", "hk4e_global", "60", "100", "0", ""]).is_ok());
    }

    #[test]
    fn refresh_delay_is_clamped() {
        let cli = p(&["run", "hk4e_global", "60", "1", "0"]).unwrap();
        if let Command::Run(r) = cli.command {
            assert_eq!(r.refresh_delay_ms, MIN_REFRESH_MS);
        }
        let cli = p(&["run", "hk4e_global", "60", "999999999", "0"]).unwrap();
        if let Command::Run(r) = cli.command {
            assert_eq!(r.refresh_delay_ms, MAX_REFRESH_MS);
        }
    }

    #[test]
    fn rejects_bad_input() {
        assert!(p(&[]).is_err());
        assert!(p(&["run"]).is_err());
        assert!(p(&["run", "bogus", "60"]).is_err());
        assert!(p(&["run", "hk4e_global", "0"]).is_err());
        assert!(p(&["run", "hk4e_global", "abc"]).is_err());
        assert!(p(&["run", "hk4e_global", "60", "0", "0", "p", "extra"]).is_err());
        assert!(p(&["--nope", "games"]).is_err());
        assert!(p(&["--log"]).is_err());
    }

    #[test]
    fn options_can_appear_anywhere() {
        let cli = p(&["--no-dialog", "run", "wuwa_global", "120", "--log", "x.log", "-v", "0", "0", "/p"]).unwrap();
        assert!(!cli.options.dialogs);
        assert!(cli.options.verbose);
        assert_eq!(cli.options.log_file, Some(PathBuf::from("x.log")));
    }

    #[test]
    fn wait_timeout_zero_means_forever() {
        let cli = p(&["--wait-timeout", "0", "games"]).unwrap();
        assert_eq!(cli.options.wait_timeout, None);
    }

    #[test]
    fn help_and_version() {
        assert!(matches!(p(&["--help"]).unwrap().command, Command::Help));
        assert!(matches!(p(&["-V"]).unwrap().command, Command::Version));
        assert!(matches!(p(&["games"]).unwrap().command, Command::Games));
    }
}
