//! Honkai: Star Rail: `HKCU\Software\Cognosphere\Star Rail`, value `GraphicsSettings_Model_h*`.
//! The game caps FPS at 120. Run BEFORE the game starts.

use super::hoyo_registry::{apply, Spec};
use crate::cli::RunArgs;
use crate::error::Result;

fn resolve(fps: u32) -> u32 {
    fps.min(120)
}

const SPEC: Spec = Spec {
    display_name: "Honkai: Star Rail",
    subkey: "Software\\Cognosphere\\Star Rail",
    value_prefix: "GraphicsSettings_Model",
    fps_fields: &["FPS"],
    resolve_fps: resolve,
};

pub fn run(args: &RunArgs) -> Result<()> {
    apply(&SPEC, args.target_fps).map(|_| ())
}
