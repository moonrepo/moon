use moon_app::EXE_NAME;
use std::env;
use std::fs;
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

    // Helper to scan a directory for the core package
    let scan_for_core = |start_dir: &Path| -> Option<PathBuf> {
        if let Ok(entries) = fs::read_dir(start_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();

                // Check for standard "core-*" or pnpm's "+core-*" naming
                if name.starts_with("core-")
                    || name.starts_with("+core-")
                    || name.starts_with("@moonrepo+core-")
                {
                    let bin_path = entry.path().join(EXE_NAME);

                    if bin_path.exists() {
                        return Some(bin_path);
                    }
                }
            }
        }
        None
    };

    while let Some(dir) = current_dir {
        if dir.join(".moon").exists() || dir.join(".config").join("moon").exists() {
            let cli_dir = dir.join("node_modules").join("@moonrepo").join("cli");

            if cli_dir.exists() {
                let mut search_paths = vec![];

                // 1. Original Symlink Path (npm, Bun, Yarn)
                // The core package might be hoisted to the root node_modules,
                // making it a sibling of the @moonrepo/cli symlink location.
                if let Some(parent) = cli_dir.parent() {
                    search_paths.push(parent.to_path_buf()); // @moonrepo/
                    if let Some(grandparent) = parent.parent() {
                        search_paths.push(grandparent.to_path_buf()); // node_modules/
                    }
                }

                // 2. Real Path (pnpm, Bun, Yarn Berry)
                // If it's a symlink, resolve it to find the real location in the store or cache.
                if let Ok(real_path) = fs::canonicalize(&cli_dir) {
                    if real_path != cli_dir {
                        search_paths.push(real_path.clone()); // Inside the package itself
                        if let Some(parent) = real_path.parent() {
                            search_paths.push(parent.to_path_buf()); // Sibling in store
                        }
                        if let Some(grandparent) = real_path.parent().and_then(|p| p.parent()) {
                            search_paths.push(grandparent.to_path_buf()); // Parent of scope
                        }
                        // Check for dependencies inside the package (unhoisted)
                        search_paths.push(real_path.join("node_modules").join("@moonrepo"));
                    }
                }

                // Scan all potential locations
                for search_path in search_paths {
                    if !search_path.exists() {
                        continue;
                    }
                    if let Some(bin) = scan_for_core(&search_path) {
                        return Some(bin);
                    }
                }
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
