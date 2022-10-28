mod node;
mod typescript;

use crate::helpers::AnyError;
use clap::ValueEnum;
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Confirm, Select};
use moon_config::{load_global_project_config_template, load_workspace_config_template};
use moon_constants::{CONFIG_DIRNAME, CONFIG_GLOBAL_PROJECT_FILENAME, CONFIG_WORKSPACE_FILENAME};
use moon_lang_node::package::{PackageJson, PackageWorkspaces};
use moon_lang_node::NPM;
use moon_logger::color;
use moon_project::detect_projects_with_globs;
use moon_terminal::create_theme;
use moon_utils::{fs, path};
use moon_vcs::detect_vcs;
use node::init_node;
use std::collections::{BTreeMap, HashMap};
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

#[derive(ValueEnum, Clone, Debug, Default)]
pub enum InheritProjectsAs {
    #[default]
    None,
    GlobsList,
    ProjectsMap,
}

impl InheritProjectsAs {
    fn get_option_index(&self) -> usize {
        match self {
            InheritProjectsAs::None => 0,
            InheritProjectsAs::GlobsList => 1,
            InheritProjectsAs::ProjectsMap => 2,
        }
    }
}

pub struct InitOptions {
    pub force: bool,
    pub inherit_projects: InheritProjectsAs,
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

/// Detect potential projects (for existing repos only) by
/// inspecting the `workspaces` field in a root `package.json`.
async fn detect_projects(
    dest_dir: &Path,
    options: &InitOptions,
) -> Result<(BTreeMap<String, String>, Vec<String>), AnyError> {
    let mut projects = HashMap::new();
    let mut project_globs = vec![];

    if let Ok(Some(pkg)) = PackageJson::read(dest_dir) {
        if let Some(workspaces) = pkg.workspaces {
            let items = vec![
                "Don't inherit",
                "As a list of globs",
                "As a map of project locations",
            ];
            let default_index = options.inherit_projects.get_option_index();

            let index = if options.yes {
                default_index
            } else {
                Select::with_theme(&create_theme())
                    .with_prompt(format!(
                        "Inherit projects from {} workspaces?",
                        color::file(NPM.manifest_filename)
                    ))
                    .items(&items)
                    .default(default_index)
                    .interact_opt()?
                    .unwrap_or(default_index)
            };

            let globs = match workspaces {
                PackageWorkspaces::Array(list) => list,
                PackageWorkspaces::Object(object) => object.packages.unwrap_or_default(),
            };

            if index == 1 {
                project_globs.extend(globs);
            } else if index == 2 {
                detect_projects_with_globs(dest_dir, &globs, &mut projects)?;
            }
        }
    }

    if projects.is_empty() && project_globs.is_empty() {
        projects.insert("example".to_owned(), "apps/example".to_owned());
    }

    // Sort the projects for template rendering
    let mut sorted_projects = BTreeMap::new();

    for (key, value) in projects {
        sorted_projects.insert(key, value);
    }

    Ok((sorted_projects, project_globs))
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
    let (projects, project_globs) = detect_projects(&dest_dir, &options).await?;
    let vcs = detect_vcs(&dest_dir).await?;

    // Create the initial files
    let mut context = Context::new();
    context.insert("projects", &projects);
    context.insert("project_globs", &project_globs);
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

    // Initialize additional languages
    if dest_dir.join(NPM.manifest_filename).exists()
        || Confirm::with_theme(&theme)
            .with_prompt("Initialize Node.js?")
            .interact()?
    {
        init_node(&dest_dir, &options, &theme).await?;

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
