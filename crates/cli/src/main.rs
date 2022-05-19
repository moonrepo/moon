use moon_cli::run_cli;
use moon_config::constants;
use std::env;
use std::path::{Path, PathBuf};
use tokio::process::Command;

/// Check whether this binary has been installed globally or not.
/// If we encounter an error, simply abort early instead of failing.
async fn is_globally_installed() -> bool {
    let exe_path = match env::current_exe() {
        Ok(path) => path,
        Err(_) => return false,
    };

    // Global installs happen *outside* of moon's toolchain,
    // so we simply assume that they have and are using npm
    // in their environment.
    let output = match Command::new("npm")
        .args(["config", "get", "prefix"])
        .output()
        .await
    {
        Ok(out) => out,
        Err(_) => return false,
    };

    // If our executable path starts with the global dir,
    // then we must have been installed globally!
    let global_dir = PathBuf::from(
        String::from_utf8(output.stdout.to_vec())
            .unwrap_or_default()
            .trim(),
    );

    exe_path.starts_with(global_dir)
}

fn find_workspace_root(dir: &Path) -> Option<PathBuf> {
    let findable = dir.join(constants::CONFIG_DIRNAME);

    if findable.exists() {
        return Some(dir.to_path_buf());
    }

    match dir.parent() {
        Some(parent_dir) => find_workspace_root(parent_dir),
        None => None,
    }
}

async fn run_bin(bin_path: &Path, current_dir: &Path) -> Result<(), std::io::Error> {
    // Remove the binary path from the current args list
    let args = env::args()
        .enumerate()
        .filter(|(i, arg)| {
            if *i == 0 {
                !arg.ends_with("moon")
            } else {
                true
            }
        })
        .map(|(_, arg)| arg)
        .collect::<Vec<String>>();

    // Execute the found moon binary with the current filtered args
    Command::new(bin_path)
        .args(args)
        .current_dir(current_dir)
        .spawn()?
        .wait()
        .await?;

    Ok(())
}

#[tokio::main]
async fn main() {
    let mut run = true;

    // Detect if we've been installed globally
    if let Ok(current_dir) = env::current_dir() {
        if is_globally_installed().await {
            println!("Global!!");

            // If so, find the workspace root so we can locate the
            // locally installed `moon` binary in node modules
            if let Some(workspace_root) = find_workspace_root(&current_dir) {
                let moon_bin = workspace_root
                    .join("node_modules")
                    .join("@moonrepo")
                    .join("cli")
                    .join("moon");

                // The binary exists! So let's run that one to ensure
                // we're running the version pinned in `package.json`,
                // instead of this global one!
                if moon_bin.exists() {
                    run = false;

                    run_bin(&moon_bin, &current_dir)
                        .await
                        .expect("Failed to run moon binary");
                }
            }
        }
    }

    // Otherwise just run the CLI
    if run {
        run_cli().await
    }
}
