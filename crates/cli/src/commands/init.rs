use moon_config::constants::{CONFIG_DIRNAME, CONFIG_PROJECT_FILENAME, CONFIG_WORKSPACE_FILENAME};
use moon_config::{load_global_project_config_template, load_workspace_config_template};
use moon_logger::color;
use moon_utils::fs;
use moon_utils::path;
use std::env;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use dialoguer::Confirm;

type AnyError = Box<dyn std::error::Error>;

/// Verify the destination and return a path to the `.moon` folder
/// if all questions have passed.
fn verify_dest(dest_dir: &Path) -> Result<Option<PathBuf>, AnyError> {
    if Confirm::new()
        .with_prompt(format!("Initialize moon into {}?", color::path(dest_dir)))
        .interact()?
    {
        let moon_dir = dest_dir.join(CONFIG_DIRNAME);

        if moon_dir.exists()
            && !Confirm::new()
                .with_prompt("Moon has already been initialized, overwrite it?")
                .interact()?
        {
            return Ok(None);
        }

        return Ok(Some(moon_dir));
    }

    Ok(None)
}

pub async fn init(dest: &str, force: bool) -> Result<(), AnyError> {
    let working_dir = env::current_dir().unwrap();
    let dest_path = PathBuf::from(dest);
    let dest_dir = if dest == "." {
        working_dir
    } else if dest_path.is_absolute() {
        dest_path
    } else {
        working_dir.join(dest)
    };

    let moon_dir = match verify_dest(&path::normalize(&dest_dir))? {
        Some(dir) => dir,
        None => return Ok(()),
    };

    // Create config files
    fs::create_dir_all(&moon_dir).await?;

    fs::write(
        &moon_dir.join(CONFIG_WORKSPACE_FILENAME),
        load_workspace_config_template(),
    )
    .await?;

    fs::write(
        &moon_dir.join(CONFIG_PROJECT_FILENAME),
        load_global_project_config_template(),
    )
    .await?;

    // Append to ignore file
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(dest_dir.join(".gitignore"))?;

    writeln!(
        file,
        r#"
# Moon
.moon/cache"#
    )?;

    println!(
        "Moon has successfully been initialized in {}",
        color::path(&dest_dir.canonicalize().unwrap()),
    );

    Ok(())
}
