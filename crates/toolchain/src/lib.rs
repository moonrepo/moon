mod errors;
mod tools;
mod traits;

use dirs::home_dir as get_home_dir;
use errors::ToolchainError;
use monolith_config::constants;
use monolith_config::WorkspaceConfig;
use std::fs;
use std::path::PathBuf;
use tools::node::NodeTool;

fn find_or_create_cache_dir() -> Result<PathBuf, ToolchainError> {
    let home_dir = match get_home_dir() {
        Some(dir) => dir,
        None => return Err(ToolchainError::MissingHomeDir),
    };

    let cache_dir = home_dir.join(constants::CONFIG_DIRNAME);

    // If path exists but is not a directory, delete it
    if cache_dir.exists() {
        if cache_dir.is_file() {
            if let Err(_) = fs::remove_file(cache_dir.as_path()) {
                return Err(ToolchainError::FailedToCreateDir);
            }
        }

        // TODO symlink

        // Otherwise attempt to create the directory
    } else {
        if let Err(_) = fs::create_dir(cache_dir.as_path()) {
            return Err(ToolchainError::FailedToCreateDir);
        }
    }

    Ok(cache_dir)
}

#[derive(Debug)]
pub struct Toolchain {
    /// The directory where tools are downloaded and installed.
    /// This is typically a folder within the user's home path.
    pub cache_dir: PathBuf,

    /// The Node.js tool instance. Will always exist.
    pub node: NodeTool,
}

impl Toolchain {
    pub fn load(config: &WorkspaceConfig) -> Result<Toolchain, ToolchainError> {
        let cache_dir = find_or_create_cache_dir()?;
        let node = NodeTool::load(&cache_dir, &config.node);

        Ok(Toolchain { cache_dir, node })
    }
}
