//! Shared logic for games whose FPS lives in a JSON blob inside a registry value.

use crate::engine::{json_patch, reg_match};
use crate::error::{Error, Result};
use crate::platform::registry::RegistryKey;
use crate::{log_debug, log_info, log_warn};

pub struct Spec {
    pub display_name: &'static str,
    /// Subkey under HKEY_CURRENT_USER.
    pub subkey: &'static str,
    /// Value-name prefix (a `_h<hash>` suffix is added by the game).
    pub value_prefix: &'static str,
    pub fps_fields: &'static [&'static str],
    /// Maps the requested FPS to what the game accepts.
    pub resolve_fps: fn(u32) -> u32,
}

pub fn apply(spec: &Spec, requested_fps: u32) -> Result<u32> {
    let key = RegistryKey::open_hkcu(spec.subkey, true).map_err(|e| match e {
        Error::NotFound(_) => Error::NotFound(format!(
            "Registry key HKCU\\{} not found. Launch {} once first.",
            spec.subkey, spec.display_name
        )),
        other => other,
    })?;

    let names = key.value_names()?;
    let name = reg_match::find_setting_value(&names, spec.value_prefix)
        .ok_or_else(|| {
            Error::NotFound(format!(
                "No '{}' value in the registry. Make sure the game uses the Custom graphics preset.",
                spec.value_prefix
            ))
        })?
        .clone();

    let (value_type, raw) = key.get_raw(&name)?;
    log_debug!("original {name}: {}", String::from_utf8_lossy(&raw).trim_end_matches('\0'));

    let mut setting = json_patch::parse_setting_bytes(&raw)?;
    if setting.sanitized {
        log_warn!("registry value needed sanitising before it could be parsed");
    }
    let fps = (spec.resolve_fps)(requested_fps);
    json_patch::set_fields(&mut setting.value, spec.fps_fields, fps)?;
    let bytes = json_patch::encode_setting(&setting.value, setting.trailing_nul)?;
    key.set_raw(&name, value_type, &bytes)?;

    log_info!("{} FPS unlocked to {fps}", spec.display_name);
    Ok(fps)
}
