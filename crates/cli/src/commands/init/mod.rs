mod node;
mod typescript;

use crate::helpers::AnyError;
use dialoguer::theme::ColorfulTheme;
use dialoguer::Confirm;
use moon_config::{load_global_project_config_template, load_workspace_config_template};
use moon_constants::{CONFIG_DIRNAME, CONFIG_GLOBAL_PROJECT_FILENAME, CONFIG_WORKSPACE_FILENAME};
use moon_lang_node::NPM;
use moon_logger::color;
use moon_terminal::create_theme;
use moon_utils::{fs, path};
use moon_vcs::detect_vcs;
use node::init_node;
use std::collections::BTreeMap;
use std::env;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use tera::{Context, Tera};
use typescript::init_typescript;

pub fn append_workspace_config(dest_dir: &Path, config: String) -> Result<(), AnyError> {
    let mut file = OpenOptions::new().create(true).append(true).open(
        dest_dir
            .join(CONFIG_DIRNAME)
            .join(CONFIG_WORKSPACE_FILENAME),
    )?;

    writeln!(file, "\n\n{}", config)?;

    Ok(())
}

pub struct InitOptions {
    pub force: bool,
    pub yes: bool,
}

/// Verify the destination and return a path to the `.moon` folder
/// if all questions have passed.
fn verify_dest_dir(
    dest_dir: &Path,
    options: &InitOptions,
    theme: &ColorfulTheme,
) -> Result<Option<PathBuf>, AnyError> {
    if options.yes
        || Confirm::with_theme(theme)
            .with_prompt(format!("Initialize moon into {}?", color::path(dest_dir)))
            .interact()?
    {
        let moon_dir = dest_dir.join(CONFIG_DIRNAME);

        if !options.force
            && moon_dir.exists()
            && !Confirm::with_theme(theme)
                .with_prompt("Moon has already been initialized, overwrite it?")
                .interact()?
        {
            return Ok(None);
        }

        return Ok(Some(moon_dir));
    }

    Ok(None)
}

pub async fn init(dest: &str, options: InitOptions) -> Result<(), AnyError> {
    let theme = create_theme();
    let working_dir = env::current_dir().expect("Failed to determine working directory.");
    let dest_path = PathBuf::from(dest);
    let dest_dir = if dest == "." {
        working_dir
    } else if dest_path.is_absolute() {
        dest_path
    } else {
        working_dir.join(dest)
    };

    // Extract template variables
    let dest_dir = path::normalize(&dest_dir);
    let moon_dir = match verify_dest_dir(&dest_dir, &options, &theme)? {
        Some(dir) => dir,
        None => return Ok(()),
    };
    // let (projects, project_globs) = detect_projects(&dest_dir, &options).await?;
    let vcs = detect_vcs(&dest_dir).await?;

    // Create the config files
    let mut context = Context::new();
    // context.insert("projects", &BTreeMap::new::<String, String>());
    // context.insert("project_globs", &vec![]);
    context.insert("vcs_manager", &vcs.0);
    context.insert("vcs_default_branch", &vcs.1);

    fs::create_dir_all(&moon_dir).await?;

    fs::write(
        &moon_dir.join(CONFIG_WORKSPACE_FILENAME),
        Tera::one_off(load_workspace_config_template(), &context, false)?,
    )
    .await?;

    fs::write(
        &moon_dir.join(CONFIG_GLOBAL_PROJECT_FILENAME),
        Tera::one_off(load_global_project_config_template(), &context, false)?,
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
.moon/cache
.moon/docker"#
    )?;

    let mut context = Context::new(); // TODO

    // Initialize additional languages
    if dest_dir.join(NPM.manifest_filename).exists()
        || Confirm::with_theme(&theme)
            .with_prompt("Initialize Node.js?")
            .interact()?
    {
        init_node(&dest_dir, &options, &mut context, &theme).await?;

        if dest_dir.join("tsconfig.json").exists()
            || Confirm::with_theme(&theme)
                .with_prompt("Initialize TypeScript?")
                .interact()?
        {
            init_typescript(&dest_dir, &options, &theme).await?;
        }
    }

    println!(
        "Moon has successfully been initialized in {}",
        color::path(&dest_dir),
    );

    Ok(())
}
