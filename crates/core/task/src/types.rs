use rustc_hash::{FxHashMap, FxHashSet};
use std::path::PathBuf;

pub type TouchedFilePaths = FxHashSet<PathBuf>;

pub type EnvVars = FxHashMap<String, String>;
