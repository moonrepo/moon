use moon_utils::path;
use std::path::PathBuf;

pub fn prepend_name(name: &str, prefix: &str) -> String {
    if prefix.is_empty() {
        return name.to_owned();
    }

    // Use native path utils to join the paths, so we can ensure
    // the parts are joined correctly within the archive!
    let parts: PathBuf = [prefix, name].iter().collect();

    path::normalize(parts).to_string_lossy().to_string()
}
