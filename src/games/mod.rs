//! One module per game + dispatch.

pub mod wuwa;
pub mod wuwa_data;

#[cfg(windows)]
mod genshin;
#[cfg(windows)]
mod honkai3;
#[cfg(windows)]
mod hoyo_registry;
#[cfg(windows)]
mod starrail;

use crate::cli::{Options, RunArgs};
#[cfg(not(windows))]
use crate::error::Error;
use crate::error::Result;
use crate::game_id::GameId;
use crate::log_info;

pub fn run(options: &Options, args: &RunArgs) -> Result<()> {
    match args.game {
        GameId::GenshinGlobal => genshin_run(options, args),
        GameId::StarRailGlobal => starrail_run(args),
        GameId::Honkai3Global => honkai3_run(args),
        GameId::WutheringWavesGlobal => wuwa::run(args),
        GameId::ZenlessGlobal => {
            log_info!("Zenless Zone Zero has a built-in unlimited FPS option; nothing to do.");
            Ok(())
        }
    }
}

#[cfg(windows)]
fn genshin_run(options: &Options, args: &RunArgs) -> Result<()> {
    genshin::run(options, args)
}
#[cfg(not(windows))]
fn genshin_run(_: &Options, _: &RunArgs) -> Result<()> {
    Err(Error::Unsupported("hk4e_global needs Windows or Wine/Proton (process memory access)"))
}

#[cfg(windows)]
fn starrail_run(args: &RunArgs) -> Result<()> {
    starrail::run(args)
}
#[cfg(not(windows))]
fn starrail_run(_: &RunArgs) -> Result<()> {
    Err(Error::Unsupported("hkrpg_global needs Windows or Wine/Proton (registry access)"))
}

#[cfg(windows)]
fn honkai3_run(args: &RunArgs) -> Result<()> {
    honkai3::run(args)
}
#[cfg(not(windows))]
fn honkai3_run(_: &RunArgs) -> Result<()> {
    Err(Error::Unsupported("bh3_global needs Windows or Wine/Proton (registry access)"))
}
