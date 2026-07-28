use starbase_utils::fs::{self, FsError};
use std::path::Path;

/// Grant the owner write permission on the file, leaving all other bits intact.
/// Reflinks clone the source's permissions, so both storing and hydrating a
/// read-only file must restore writability on the clone.
pub fn grant_owner_write_access(path: &Path) -> miette::Result<()> {
    let mut perms = fs::metadata(path)?.permissions();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        perms.set_mode(perms.mode() | 0o200);
    }

    // The readonly attribute is the only permission Windows has
    #[cfg(not(unix))]
    #[allow(clippy::permissions_set_readonly_false)]
    perms.set_readonly(false);

    std::fs::set_permissions(path, perms).map_err(|error| FsError::Perms {
        path: path.to_path_buf(),
        error: Box::new(error),
    })?;

    Ok(())
}
