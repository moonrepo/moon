use miette::IntoDiagnostic;
pub use relative_path::*;
use rustc_hash::FxHasher;
use starbase_styles::color;
use std::hash::Hasher;
use std::path::{Path, PathBuf};

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

pub fn clean_components<T: AsRef<Path>>(path: T) -> PathBuf {
    use std::path::Component;

    // Based on https://gitlab.com/foo-jin/clean-path
    let mut components = path.as_ref().components().peekable();

    let mut cleaned = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
        components.next();

        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    let mut leading_parent_dots = 0;
    let mut component_count = 0;

    for component in components {
        match component {
            Component::Prefix(_) | Component::CurDir => {}
            Component::RootDir => {
                cleaned.push(component.as_os_str());
                component_count += 1;
            }
            Component::ParentDir => {
                if component_count == 1 && cleaned.is_absolute() {
                    // Nothing
                } else if component_count == leading_parent_dots {
                    cleaned.push("..");
                    leading_parent_dots += 1;
                    component_count += 1;
                } else {
                    cleaned.pop();
                    component_count -= 1;
                }
            }
            Component::Normal(c) => {
                cleaned.push(c);
                component_count += 1;
            }
        }
    }

    if component_count == 0 {
        cleaned.push(".");
    }

    cleaned
}

pub fn paths_are_equal<L: AsRef<Path>, R: AsRef<Path>>(left: L, right: R) -> bool {
    clean_components(left) == clean_components(right)
}
