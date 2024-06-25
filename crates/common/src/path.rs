pub use relative_path::*;
use starbase_styles::color;
use std::path::Path;

// Named types for better readability
pub type ProjectRelativePath = RelativePath;
pub type ProjectRelativePathBuf = RelativePathBuf;
pub type WorkspaceRelativePath = RelativePath;
pub type WorkspaceRelativePathBuf = RelativePathBuf;

#[cfg(unix)]
#[inline]
pub fn normalize_separators<T: AsRef<str>>(path: T) -> String {
    path.as_ref().replace('\\', "/")
}

#[cfg(windows)]
#[inline]
pub fn normalize_separators<T: AsRef<str>>(path: T) -> String {
    path.as_ref().replace('/', "\\")
}

#[inline]
pub fn standardize_separators<T: AsRef<str>>(path: T) -> String {
    path.as_ref().replace('\\', "/")
}

pub enum RelativeFrom<'path> {
    Project(&'path str),
    Workspace,
}

pub fn expand_to_workspace_relative<P: AsRef<str>>(
    from_format: RelativeFrom,
    path: P,
) -> WorkspaceRelativePathBuf {
    let path = standardize_separators(path.as_ref());

    match from_format {
        RelativeFrom::Project(source) => {
            // Root-level project
            if source.is_empty() || source == "." {
                WorkspaceRelativePathBuf::from(path)

                // Project-level, prefix with source path
            } else {
                let project_source = standardize_separators(source);

                if let Some(negated_glob) = path.strip_prefix('!') {
                    WorkspaceRelativePathBuf::from(format!("!{project_source}")).join(negated_glob)
                } else {
                    WorkspaceRelativePathBuf::from(project_source).join(path)
                }
            }
        }
        RelativeFrom::Workspace => WorkspaceRelativePathBuf::from(path),
    }
}

#[inline]
pub fn to_string<T: AsRef<Path>>(path: T) -> miette::Result<String> {
    let path = path.as_ref();

    match path.to_str() {
        Some(p) => Ok(p.to_owned()),
        None => Err(miette::miette!(
            "Path {} contains invalid UTF-8 characters.",
            color::path(path)
        )),
    }
}

#[inline]
pub fn to_virtual_string<T: AsRef<Path>>(path: T) -> miette::Result<String> {
    Ok(standardize_separators(to_string(path)?))
}

#[inline]
pub fn exe_name<T: AsRef<str>>(name: T) -> String {
    #[cfg(windows)]
    {
        format!("{}.exe", name.as_ref())
    }

    #[cfg(not(windows))]
    {
        name.as_ref().into()
    }
}
