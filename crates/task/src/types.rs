use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

pub type TouchedFilePaths = HashSet<PathBuf>;

pub type EnvVars = HashMap<String, String>;
