use moon_config::constants::{CONFIG_DIRNAME, CONFIG_PROJECT_FILENAME, CONFIG_WORKSPACE_FILENAME};
use moon_config::{load_global_project_config_template, load_workspace_config_template};
use moon_logger::color;
use moon_utils::fs;
use std::env;

pub async fn init(dest: &str, force: bool) -> Result<(), Box<dyn std::error::Error>> {
    let working_dir = env::current_dir().unwrap();
    let dest_dir = working_dir.join(dest).canonicalize().unwrap();
    let moon_dir = dest_dir.join(CONFIG_DIRNAME);

    if moon_dir.exists() && !force {
        println!(
            "Moon has already been initialized in {} (pass {} to overwrite)",
            color::file_path(&dest_dir),
            color::shell("--force")
        );

        return Ok(());
    }

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

    println!(
        "Moon has successfully been initialized in {}",
        color::file_path(&dest_dir),
    );

    Ok(())
}
