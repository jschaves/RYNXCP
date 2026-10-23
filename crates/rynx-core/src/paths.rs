//! Filesystem helpers that refuse to escape their managed root and never
//! follow symlinks when writing privileged configuration.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::{crypto, CoreError, CoreResult};

/// Join a validated relative path to `base`, refusing traversal.
pub fn safe_join(base: &Path, relative: &str) -> CoreResult<PathBuf> {
    crate::validate::validate_relative_path(relative, "path")?;
    let joined = base.join(relative);
    if !joined.starts_with(base) {
        return Err(CoreError::Validation("path escapes its root".into()));
    }
    Ok(joined)
}

fn canonical_base(base: &Path) -> CoreResult<PathBuf> {
    fs::create_dir_all(base)
        .map_err(|e| CoreError::Io(format!("cannot create {}: {e}", base.display())))?;
    fs::canonicalize(base)
        .map_err(|e| CoreError::Io(format!("cannot resolve {}: {e}", base.display())))
}

fn assert_within(base_canon: &Path, candidate: &Path) -> CoreResult<()> {
    let parent = candidate
        .parent()
        .ok_or_else(|| CoreError::Io("path has no parent".into()))?;
    let parent_canon = fs::canonicalize(parent)
        .map_err(|e| CoreError::Io(format!("cannot resolve {}: {e}", parent.display())))?;
    if !parent_canon.starts_with(base_canon) {
        return Err(CoreError::Validation(format!(
            "refusing to write outside {}",
            base_canon.display()
        )));
    }
    Ok(())
}

/// Atomically write a managed file below `base`. Replaces symlinks rather
/// than following them and sets restrictive permissions.
pub fn write_managed(
    base: &Path,
    path: &Path,
    content: &str,
    mode: u32,
    owner: Option<(u32, u32)>,
) -> CoreResult<()> {
    let base_canon = canonical_base(base)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| CoreError::Io(format!("cannot create {}: {e}", parent.display())))?;
    }
    assert_within(&base_canon, path)?;

    let temp = path.with_file_name(format!(".rynx-{}.tmp", crypto::random_hex(6)));
    {
        let mut opts = OpenOptions::new();
        opts.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.custom_flags(libc::O_NOFOLLOW).mode(mode);
        }
        let mut file = opts
            .open(&temp)
            .map_err(|e| CoreError::Io(format!("cannot create {}: {e}", temp.display())))?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temp, fs::Permissions::from_mode(mode))?;
        if let Some((uid, gid)) = owner {
            std::os::unix::fs::chown(&temp, Some(uid), Some(gid))
                .map_err(|e| CoreError::Io(format!("chown failed: {e}")))?;
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (mode, owner);
    }

    fs::rename(&temp, path).map_err(|e| {
        let _ = fs::remove_file(&temp);
        CoreError::Io(format!("cannot install {}: {e}", path.display()))
    })?;
    Ok(())
}

pub fn remove_managed(base: &Path, path: &Path) -> CoreResult<()> {
    let base_canon = canonical_base(base)?;
    if !path.exists() && fs::symlink_metadata(path).is_err() {
        return Ok(());
    }
    assert_within(&base_canon, path)?;
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_dir() => fs::remove_dir_all(path)?,
        Ok(_) => fs::remove_file(path)?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    Ok(())
}

pub fn ensure_dir(
    base: &Path,
    path: &Path,
    mode: u32,
    owner: Option<(u32, u32)>,
) -> CoreResult<()> {
    let base_canon = canonical_base(base)?;
    fs::create_dir_all(path)?;
    assert_within(&base_canon, path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
        if let Some((uid, gid)) = owner {
            let _ = std::os::unix::fs::chown(path, Some(uid), Some(gid));
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (mode, owner);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_rejects_traversal() {
        let base = Path::new("/tmp/rynx-test");
        assert!(safe_join(base, "public").is_ok());
        assert!(safe_join(base, "../etc/passwd").is_err());
        assert!(safe_join(base, "/etc/passwd").is_err());
    }

    #[test]
    fn write_is_confined() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("managed");
        fs::create_dir_all(&base).unwrap();
        let target = base.join("nginx/site.conf");
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        write_managed(&base, &target, "server {}\n", 0o644, None).unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), "server {}\n");
        assert!(write_managed(&base, &dir.path().join("escape.conf"), "x", 0o644, None).is_err());
    }
}
