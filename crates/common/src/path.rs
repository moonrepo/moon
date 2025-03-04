use miette::IntoDiagnostic;
pub use relative_path::*;
use rustc_hash::FxHasher;
use starbase_styles::color;
use std::hash::Hasher;
use std::path::Path;

// Named types for better readability
pub type ProjectRelativePath = RelativePath;
pub type ProjectRelativePathBuf = RelativePathBuf;
pub type WorkspaceRelativePath = RelativePath;
pub type WorkspaceRelativePathBuf = RelativePathBuf;

#[inline]
pub fn is_root_level_source<T: AsRef<str>>(source: T) -> bool {
    let source = source.as_ref();
    source.is_empty() || source == "."
}

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
            if is_root_level_source(source) {
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
pub fn to_relative_virtual_string<F: AsRef<Path>, T: AsRef<Path>>(
    from: F,
    to: T,
) -> miette::Result<String> {
    let value = from
        .as_ref()
        .relative_to(to.as_ref())
        .into_diagnostic()?
        .to_string();

    Ok(if value.is_empty() { ".".into() } else { value })
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

/// Encode a value (typically an identifier) by removing invalid characters for use
/// within a file name.
pub fn encode_component(value: impl AsRef<str>) -> String {
    let mut output = String::new();

    // Handle supported characters from `Id`
    for ch in value.as_ref().chars() {
        match ch {
            '@' | '*' => {
                // Skip these
            }
            '/' | ':' => {
                output.push('-');
            }
            _ => {
                output.push(ch);
            }
        }
    }

    output.trim_matches(['-', '.']).to_owned()
}

/// Hash a value that may contain special characters into a valid file name.
pub fn hash_component(value: impl AsRef<str>) -> String {
    let mut hasher = FxHasher::default();
    hasher.write(value.as_ref().as_bytes());

    format!("{}", hasher.finish())
}
