use clap::ValueEnum;
use dialoguer::{Confirm, Select};
use moon_config::{
    default_node_version, default_npm_version, default_pnpm_version, default_yarn_version,
    load_global_project_config_template, load_workspace_config_template,
};
use moon_constants::{CONFIG_DIRNAME, CONFIG_GLOBAL_PROJECT_FILENAME, CONFIG_WORKSPACE_FILENAME};
use moon_lang::{is_using_package_manager, is_using_version_manager};
use moon_lang_node::package::{PackageJson, PackageWorkspaces};
use moon_lang_node::{NODENV, NPM, NVMRC, PNPM, YARN};
use moon_logger::color;
use moon_project::detect_projects_with_globs;
use moon_terminal::create_theme;
use moon_utils::{fs, path};
use moon_vcs::detect_vcs;
use std::collections::{BTreeMap, HashMap};
use std::env;
use std::fs::{read_to_string, OpenOptions};
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use tera::{Context, Tera};

#[derive(ValueEnum, Clone, Debug, Default)]
pub enum PackageManager {
    #[default]
    Npm,
    Pnpm,
    Yarn,
}

impl PackageManager {
    fn get_option_index(&self) -> usize {
        match self {
            PackageManager::Npm => 0,
            PackageManager::Pnpm => 1,
            PackageManager::Yarn => 2,
        }
    }
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
    pub package_manager: PackageManager,
    pub yes: bool,
}

type AnyError = Box<dyn std::error::Error>;

/// Verify the destination and return a path to the `.moon` folder
/// if all questions have passed.
fn verify_dest_dir(dest_dir: &Path, options: &InitOptions) -> Result<Option<PathBuf>, AnyError> {
    let theme = create_theme();

    if options.yes
        || Confirm::with_theme(&theme)
            .with_prompt(format!("Initialize moon into {}?", color::path(dest_dir)))
            .interact()?
    {
        let moon_dir = dest_dir.join(CONFIG_DIRNAME);

        if !options.force
            && moon_dir.exists()
            && !Confirm::with_theme(&theme)
                .with_prompt("Moon has already been initialized, overwrite it?")
                .interact()?
        {
            return Ok(None);
        }

        return Ok(Some(moon_dir));
    }

    Ok(None)
}

/// Verify the package manager to use. If a `package.json` exists,
/// and the `packageManager` field is defined, use that.
async fn detect_package_manager(
    dest_dir: &Path,
    options: &InitOptions,
) -> Result<(String, String), AnyError> {
    let mut pm_type = String::new();
    let mut pm_version = String::new();

    // Extract value from `packageManager` field
    if let Ok(Some(pkg)) = PackageJson::read(dest_dir) {
        if let Some(pm) = pkg.package_manager {
            if pm.contains('@') {
                let mut parts = pm.split('@');

                pm_type = parts.next().unwrap_or_default().to_owned();
                pm_version = parts.next().unwrap_or_default().to_owned();
            } else {
                pm_type = pm;
            }
        }
    }

    // If no value, detect based on files
    if pm_type.is_empty() {
        if is_using_package_manager(dest_dir, &YARN) {
            pm_type = YARN.binary.to_owned();
        } else if is_using_package_manager(dest_dir, &PNPM) {
            pm_type = PNPM.binary.to_owned();
        } else if is_using_package_manager(dest_dir, &NPM) {
            pm_type = NPM.binary.to_owned();
        }
    }

    // If no value again, ask for explicit input
    if pm_type.is_empty() {
        let items = vec![NPM.binary, PNPM.binary, YARN.binary];
        let default_index = options.package_manager.get_option_index();

        let index = if options.yes {
            default_index
        } else {
            Select::with_theme(&create_theme())
                .with_prompt("Which package manager?")
                .items(&items)
                .default(default_index)
                .interact_opt()?
                .unwrap_or(default_index)
        };

        pm_type = String::from(items[index]);
    }

    // If no version, fallback to configuration default
    if pm_version.is_empty() {
        if pm_type == NPM.binary {
            pm_version = default_npm_version();
        } else if pm_type == PNPM.binary {
            pm_version = default_pnpm_version();
        } else if pm_type == YARN.binary {
            pm_version = default_yarn_version();
        }
    }

    Ok((pm_type, pm_version))
}

/// Detect the Node.js version from local configuration files,
/// otherwise fallback to the configuration default.
fn detect_node_version(dest_dir: &Path) -> Result<(String, String), AnyError> {
    if is_using_version_manager(dest_dir, &NVMRC) {
        return Ok((
            read_to_string(dest_dir.join(NVMRC.version_filename))?
                .trim()
                .to_owned(),
            NVMRC.binary.to_owned(),
        ));
    }

    if is_using_version_manager(dest_dir, &NODENV) {
        return Ok((
            read_to_string(dest_dir.join(NODENV.version_filename))?
                .trim()
                .to_owned(),
            NODENV.binary.to_owned(),
        ));
    }

    Ok((default_node_version(), String::new()))
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
    let working_dir = env::current_dir().unwrap();
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
    let moon_dir = match verify_dest_dir(&dest_dir, &options)? {
        Some(dir) => dir,
        None => return Ok(()),
    };
    let package_manager = detect_package_manager(&dest_dir, &options).await?;
    let node_version = detect_node_version(&dest_dir)?;
    let (projects, project_globs) = detect_projects(&dest_dir, &options).await?;
    let vcs = detect_vcs(&dest_dir).await?;

    // Generate a template
    let mut context = Context::new();
    context.insert("package_manager", &package_manager.0);
    context.insert("package_manager_version", &package_manager.1);
    context.insert("node_version", &node_version.0);
    context.insert("node_version_manager", &node_version.1);
    context.insert("projects", &projects);
    context.insert("project_globs", &project_globs);
    context.insert("vcs_manager", &vcs.0.to_string());
    context.insert("vcs_default_branch", &vcs.1);

    let mut tera = Tera::default();
    tera.add_raw_template("workspace", load_workspace_config_template())?;
    tera.add_raw_template("project", load_global_project_config_template())?;

    // Create config files
    fs::create_dir_all(&moon_dir).await?;

    fs::write(
        &moon_dir.join(CONFIG_WORKSPACE_FILENAME),
        tera.render("workspace", &context)?,
    )
    .await?;

    fs::write(
        &moon_dir.join(CONFIG_GLOBAL_PROJECT_FILENAME),
        tera.render("project", &context)?,
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

    println!(
        "Moon has successfully been initialized in {}",
        color::path(&dest_dir),
    );

    Ok(())
}
