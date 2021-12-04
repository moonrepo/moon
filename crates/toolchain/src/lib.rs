mod errors;
mod tool;
mod tools;

use dirs::home_dir as get_home_dir;
use errors::ToolchainError;
use monolith_config::constants;
use std::fs;
use std::path::PathBuf;

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
    dir: Option<PathBuf>,

    temp_dir: Option<PathBuf>,

    tools_dir: Option<PathBuf>,
}

impl Toolchain {
    /// Returns the directory where toolchain artifacts are stored.
    /// This is typically ~/.monolith.
    fn get_dir(&self) -> Result<PathBuf, ToolchainError> {
        match self.dir {
            Some(dir) => Ok(dir),
            None => {
                let home_dir = get_home_dir().ok_or(ToolchainError::MissingHomeDir)?;
                let cache_dir = home_dir.join(constants::CONFIG_DIRNAME);

                create_dir(&cache_dir)?;

                self.dir = Some(cache_dir);

                return Ok(cache_dir);
            }
        }
    }

    /// Returns the directory where temporary files are stored.
    /// This is typically ~/.monolith/temp.
    fn get_temp_dir(&self) -> Result<PathBuf, ToolchainError> {
        match self.temp_dir {
            Some(dir) => Ok(dir),
            None => {
                let temp_dir = self.get_dir()?.join("temp");

                create_dir(&temp_dir)?;

                self.temp_dir = Some(temp_dir);

                return Ok(temp_dir);
            }
        }
    }

    /// Returns the directory where tools are installed by version.
    /// This is typically ~/.monolith/tools.
    fn get_tools_dir(&self) -> Result<PathBuf, ToolchainError> {
        match self.temp_dir {
            Some(dir) => Ok(dir),
            None => {
                let tools_dir = self.get_dir()?.join("tools");

                create_dir(&tools_dir)?;

                self.tools_dir = Some(tools_dir);

                return Ok(tools_dir);
            }
        }
    }
}
