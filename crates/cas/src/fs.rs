use crate::cas_error::CasError;
use std::fs::{self, Permissions};
use std::path::Path;

pub fn mark_writable(path: &Path) -> miette::Result<()> {
    let perms;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        perms = Permissions::from_mode(0o644);
    }

    #[cfg(not(unix))]
    {
        perms = fs::metadata(path)
            .map_err(|error| CasError::ReadFailed {
                path: path.to_owned(),
                error: Box::new(error),
            })?
            .permissions();
        perms.set_readonly(false);
    }

    fs::set_permissions(path, perms).map_err(|error| CasError::WriteFailed {
        path: path.to_owned(),
        error: Box::new(error),
    })?;

    Ok(())
}

pub fn mark_readonly(path: &Path) -> miette::Result<()> {
    let perms;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        perms = Permissions::from_mode(0o444);
    }

    #[cfg(not(unix))]
    {
        perms = fs::metadata(path)
            .map_err(|error| CasError::ReadFailed {
                path: path.to_owned(),
                error: Box::new(error),
            })?
            .permissions();
        perms.set_readonly(true);
    }

    fs::set_permissions(path, perms).map_err(|error| CasError::WriteFailed {
        path: path.to_owned(),
        error: Box::new(error),
    })?;

    Ok(())
}
