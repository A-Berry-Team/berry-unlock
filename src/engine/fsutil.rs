//! Small, safe filesystem helpers: one-time backups and atomic replacement.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

pub fn sibling_with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().map(|n| n.to_os_string()).unwrap_or_default();
    name.push(suffix);
    path.with_file_name(name)
}

/// Copies `path` to `<path>.berry.bak` unless that backup already exists, so the
/// very first (pristine) version is the one that is kept.
pub fn backup_once(path: &Path) -> Result<Option<PathBuf>> {
    let bak = sibling_with_suffix(path, ".berry.bak");
    if bak.exists() {
        return Ok(None);
    }
    fs::copy(path, &bak).map_err(|e| Error::io(format!("backing up {}", path.display()), e))?;
    Ok(Some(bak))
}

/// Writes to a temp file next to `path`, fsyncs, then renames over the target.
pub fn atomic_write(path: &Path, data: &[u8]) -> Result<()> {
    let tmp = sibling_with_suffix(path, ".berry.tmp");
    let write = || -> std::io::Result<()> {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(data)?;
        f.sync_all()
    };
    if let Err(e) = write() {
        let _ = fs::remove_file(&tmp);
        return Err(Error::io(format!("writing {}", tmp.display()), e));
    }
    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        Error::io(format!("replacing {}", path.display()), e)
    })
}

pub fn require_file(path: &Path) -> Result<()> {
    match fs::metadata(path) {
        Ok(m) if m.is_file() => Ok(()),
        _ => Err(Error::NotFound(format!("{} does not exist", path.display()))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("berry_fsutil_{}_{}", std::process::id(), name))
    }

    #[test]
    fn atomic_write_replaces_and_leaves_no_temp() {
        let p = temp("a.txt");
        fs::write(&p, b"old").unwrap();
        atomic_write(&p, b"new").unwrap();
        assert_eq!(fs::read(&p).unwrap(), b"new");
        assert!(!sibling_with_suffix(&p, ".berry.tmp").exists());
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn backup_keeps_the_first_version_only() {
        let p = temp("b.txt");
        fs::write(&p, b"v1").unwrap();
        assert!(backup_once(&p).unwrap().is_some());
        fs::write(&p, b"v2").unwrap();
        assert!(backup_once(&p).unwrap().is_none());
        assert_eq!(fs::read(sibling_with_suffix(&p, ".berry.bak")).unwrap(), b"v1");
        let _ = fs::remove_file(&p);
        let _ = fs::remove_file(sibling_with_suffix(&p, ".berry.bak"));
    }

    #[test]
    fn require_file_rejects_missing_and_directories() {
        assert!(require_file(&temp("nope")).is_err());
        assert!(require_file(&std::env::temp_dir()).is_err());
    }
}
