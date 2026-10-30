//! Top-level orchestration: parse -> init logging -> dispatch -> report.

use crate::cli::{self, Command, HELP};
use crate::error::Error;
use crate::game_id::GameId;
use crate::{games, log_error, log_info, logging, ui};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
const NOTICE_TITLE: &str = "BerryUnlock notice";

fn games_text() -> String {
    let mut s = String::from("Available game IDs:\n");
    for g in GameId::ALL {
        s.push_str(&format!(" - {} = {}\n", g.display_name(), g.id()));
    }
    s
}

/// Runs the program and returns the process exit code.
pub fn run<I, S>(args: I) -> i32
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let args: Vec<String> = args.into_iter().map(Into::into).collect();
    let quiet = args.iter().any(|a| a == "--no-dialog");

    let cli = match cli::parse(args) {
        Ok(c) => c,
        Err(e) => {
            ui::notice(!quiet, NOTICE_TITLE, &e.to_string());
            return e.exit_code();
        }
    };

    logging::init(cli.options.log_file.as_deref(), cli.options.verbose);
    std::panic::set_hook(Box::new(|info| {
        log_error!("internal error: {info}");
    }));

    match cli.command {
        Command::Help => {
            ui::print_info(cli.options.dialogs, HELP);
            0
        }
        Command::Version => {
            ui::print_info(cli.options.dialogs, &format!("berry_unlock {VERSION}"));
            0
        }
        Command::Games => {
            ui::print_info(cli.options.dialogs, &games_text());
            0
        }
        Command::Run(args) => {
            log_info!(
                "BerryUnlock {VERSION}: game={} fps={} refresh={}ms init={}ms",
                args.game.id(),
                args.target_fps,
                args.refresh_delay_ms,
                args.init_delay_ms
            );
            match games::run(&cli.options, &args) {
                Ok(()) => 0,
                Err(e) => report(&cli.options, &e),
            }
        }
    }
}

fn report(options: &cli::Options, e: &Error) -> i32 {
    log_error!("{e}");
    if !e.is_silent() {
        ui::notice(options.dialogs, NOTICE_TITLE, &e.user_message());
    }
    e.exit_code()
}
