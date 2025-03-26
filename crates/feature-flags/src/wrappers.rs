use crate::{FeatureFlags, Flag};
use starbase_utils::glob::{self, GlobError, GlobWalkOptions};
use std::fmt::Debug;
use std::path::{Path, PathBuf};

pub fn glob_walk<'glob, P, I, V>(base_dir: P, patterns: I) -> Result<Vec<PathBuf>, GlobError>
where
    P: AsRef<Path> + Debug,
    I: IntoIterator<Item = &'glob V> + Debug,
    V: AsRef<str> + 'glob + ?Sized + Debug,
{
    glob_walk_with_options(base_dir, patterns, GlobWalkOptions::default())
}

pub fn glob_walk_with_options<'glob, P, I, V>(
    base_dir: P,
    patterns: I,
    options: GlobWalkOptions,
) -> Result<Vec<PathBuf>, GlobError>
where
    P: AsRef<Path> + Debug,
    I: IntoIterator<Item = &'glob V> + Debug,
    V: AsRef<str> + 'glob + ?Sized + Debug,
{
    if FeatureFlags::instance().is_enabled(Flag::FastGlobWalk) {
        glob::walk_fast_with_options(base_dir, patterns, options)
    } else if options.only_files {
        glob::walk_files(base_dir, patterns)
    } else {
        glob::walk(base_dir, patterns)
    }
}
