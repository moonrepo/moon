use moon_common::consts::{BIN_NAME, CONFIG_DIRNAME};
use std::env;
use std::path::{Path, PathBuf};

#[cfg(unix)]
fn get_global_lookups(home_dir: &Path) -> Vec<PathBuf> {
    vec![
        "/usr/local/bin".into(),
        home_dir.join(".moon"),
        home_dir.join(".proto"),
        // Node
        home_dir.join(".nvm/versions/node"),
        home_dir.join(".nodenv/versions"),
        home_dir.join(".fnm/node-versions"),
        home_dir.join("Library/pnpm"),
        home_dir.join(".local/share/pnpm"),
        home_dir.join(".config/yarn"),
    ]
}

#[cfg(windows)]
fn get_global_lookups(home_dir: &Path) -> Vec<PathBuf> {
    vec![
        home_dir.join(".moon"),
        home_dir.join(".proto"),
        // Node
        home_dir.join(".nvm\\versions\\node"),
        home_dir.join(".nodenv\\versions"),
        home_dir.join(".fnm\\node-versions"),
        home_dir.join("AppData\\npm"),
        home_dir.join("AppData\\Roaming\\npm"),
        home_dir.join("AppData\\Local\\pnpm"),
        home_dir.join("AppData\\Yarn\\config"),
    ]
}

/// Check whether this binary has been installed globally or not.
/// If we encounter an error, simply abort early instead of failing.
pub fn is_globally_installed(home_dir: &Path) -> bool {
    let exe_path = match env::current_exe() {
        Ok(path) => path,
        Err(_) => return false,
    };

    // If our executable path starts with the global dir,
    // then we must have been installed globally!
    get_global_lookups(home_dir)
        .iter()
        .any(|lookup| exe_path.starts_with(lookup))
}

pub fn has_locally_installed(home_dir: &Path, current_dir: &Path) -> Option<PathBuf> {
    let mut current_dir = Some(current_dir);

    while let Some(dir) = current_dir {
        if dir.join(CONFIG_DIRNAME).exists() {
            let cli_bin = dir
                .join("node_modules")
                .join("@moonrepo")
                .join("cli")
                .join(BIN_NAME);

            if cli_bin.exists() {
                return Some(cli_bin);
            }
        }

        if dir == home_dir {
            break;
        } else {
            current_dir = dir.parent();
        }
    }

    None
}
