//! Honkai Impact 3rd: `HKCU\Software\miHoYo\Honkai Impact 3rd`, value `PersonalGraphicsSettingV2_h*`.
//! Values >= 500 are treated as "no unlock" and mapped to 60 (KeqingUnlock behaviour).
//! Run BEFORE the game starts.

use super::hoyo_registry::{apply, Spec};
use crate::cli::RunArgs;
use crate::error::Result;

fn resolve(fps: u32) -> u32 {
    if fps >= 500 { 60 } else { fps }
}

const SPEC: Spec = Spec {
    display_name: "Honkai Impact 3rd",
    subkey: "Software\\miHoYo\\Honkai Impact 3rd",
    value_prefix: "PersonalGraphicsSettingV2",
    fps_fields: &["TargetFrameRateForInLevel", "TargetFrameRateForOthers"],
    resolve_fps: resolve,
};

pub fn run(args: &RunArgs) -> Result<()> {
    apply(&SPEC, args.target_fps).map(|_| ())
}
