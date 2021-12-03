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

fn create_dir(dir: &PathBuf) -> Result<(), ToolchainError> {
    // If path exists but is not a directory, delete it
    if dir.exists() {
        if dir.is_file() {
            if let Err(_) = fs::remove_file(dir.as_path()) {
                return Err(ToolchainError::FailedToCreateDir);
            }
        }

        // TODO symlink

        // Otherwise attempt to create the directory
    } else {
        if let Err(_) = fs::create_dir(dir.as_path()) {
            return Err(ToolchainError::FailedToCreateDir);
        }
    }

    Ok(())
}

fn find_or_create_cache_dir() -> Result<PathBuf, ToolchainError> {
    let home_dir = match get_home_dir() {
        Some(dir) => dir,
        None => return Err(ToolchainError::MissingHomeDir),
    };

    let cache_dir = home_dir.join(constants::CONFIG_DIRNAME);

    create_dir(&cache_dir)?;

    Ok(cache_dir)
}

fn find_or_create_temp_dir(cache_dir: &PathBuf) -> Result<PathBuf, ToolchainError> {
    let temp_dir = cache_dir.join("temp");

    create_dir(&cache_dir)?;

    Ok(temp_dir)
}

#[derive(Debug)]
pub struct Toolchain {
    /// The directory where tools are downloaded and installed.
    /// This is typically ~/.monolith.
    pub cache_dir: PathBuf,

    /// The directory where temporary files are stored.
    /// This is typically ~/.monolith/temp.
    pub temp_dir: PathBuf,

    /// The Node.js tool instance. Will always exist.
    pub node: NodeTool,
}

impl Toolchain {
    pub fn load(config: &WorkspaceConfig) -> Result<Toolchain, ToolchainError> {
        let cache_dir = find_or_create_cache_dir()?;
        let temp_dir = find_or_create_temp_dir(&cache_dir)?;
        let node = NodeTool::load(&cache_dir, &config.node);

        Ok(Toolchain {
            cache_dir,
            temp_dir,
            node,
        })
    }
}
