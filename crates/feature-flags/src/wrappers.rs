use crate::{FeatureFlags, Flag};
use starbase_utils::glob::{self, GlobError};
use std::fmt::Debug;
use std::path::{Path, PathBuf};

pub fn glob_walk<'glob, P, I, V>(
    base_dir: P,
    patterns: I,
    only_files: bool,
) -> Result<Vec<PathBuf>, GlobError>
where
    P: AsRef<Path> + Debug,
    I: IntoIterator<Item = &'glob V> + Debug,
    V: AsRef<str> + 'glob + ?Sized,
{
    if FeatureFlags::session().is_enabled(Flag::FastGlobWalk) {
        if only_files {
            glob::walk_fast(base_dir, patterns)
        } else {
            glob::walk_files_fast(base_dir, patterns)
        }
    } else {
        if only_files {
            glob::walk(base_dir, patterns)
        } else {
            glob::walk_files(base_dir, patterns)
        }
    }
}
