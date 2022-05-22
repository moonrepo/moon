use std::path::{Path, PathBuf};
use wax::{Glob, Pattern};

/// Wax currently doesn't support negated globs (starts with !),
/// so we must extract them manually.
pub fn split_patterns(patterns: &[String]) -> (Vec<String>, Vec<String>) {
    let mut expressions = vec![];
    let mut negations = vec![];

    for pattern in patterns {
        if pattern.starts_with('!') {
            negations.push(pattern.strip_prefix('!').unwrap().to_owned());
        } else {
            expressions.push(pattern.clone());
        }
    }

    (expressions, negations)
}

pub fn walk(base_dir: &Path, patterns: &[String]) -> Vec<PathBuf> {
    let (expressions, _negations) = split_patterns(patterns);
    let mut paths = vec![];

    for expression in expressions {
        let glob = Glob::new(&expression).unwrap();

        for entry in glob.walk(base_dir, usize::MAX)
        // .not(&negations)
        {
            match entry {
                Ok(e) => paths.push(e.into_path()),
                Err(_) => {
                    // Will crash if the file doesnt exist
                    continue;
                }
            };
        }
    }

    paths
}
