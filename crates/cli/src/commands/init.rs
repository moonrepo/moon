use dialoguer::{Confirm, Select};
use moon_config::constants::{CONFIG_DIRNAME, CONFIG_PROJECT_FILENAME, CONFIG_WORKSPACE_FILENAME};
use moon_config::package::{PackageJson, Workspaces};
use moon_config::{
    default_node_version, default_npm_version, default_pnpm_version, default_yarn_version,
    load_global_project_config_template, load_workspace_config_template,
};
use moon_lang::is_using_package_manager;
use moon_lang_node::{NODENV, NPM, NVMRC, PNPM, YARN};
use moon_logger::color;
use moon_project::{detect_projects_with_globs, ProjectsSourceMap};
use moon_terminal::create_theme;
use moon_utils::{fs, path};
use std::collections::HashMap;
use std::env;
use std::fs::{read_to_string, OpenOptions};
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use tera::{Context, Tera};

type AnyError = Box<dyn std::error::Error>;

/// Verify the destination and return a path to the `.moon` folder
/// if all questions have passed.
fn verify_dest_dir(dest_dir: &Path, yes: bool, force: bool) -> Result<Option<PathBuf>, AnyError> {
    let theme = create_theme();

    if yes
        || Confirm::with_theme(&theme)
            .with_prompt(format!("Initialize moon into {}?", color::path(dest_dir)))
            .interact()?
    {
        let moon_dir = dest_dir.join(CONFIG_DIRNAME);

        if !force
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
async fn detect_package_manager(dest_dir: &Path, yes: bool) -> Result<(String, String), AnyError> {
    let pkg_path = dest_dir.join("package.json");
    let mut pm_type = String::new();
    let mut pm_version = String::new();

    // Extract value from `packageManager` field
    if pkg_path.exists() {
        if let Ok(pkg) = PackageJson::load(&pkg_path).await {
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
    }

    // If no value, detect based on files
    if pm_type.is_empty() {
        if is_using_package_manager(dest_dir, &YARN) {
            pm_type = String::from("yarn");
        } else if is_using_package_manager(dest_dir, &PNPM) {
            pm_type = String::from("pnpm");
        } else if is_using_package_manager(dest_dir, &NPM) {
            pm_type = String::from("npm");
        }
    }

    // If no value again, ask for explicit input
    if pm_type.is_empty() {
        if yes {
            pm_type = String::from("npm");
        } else {
            let items = vec!["npm", "pnpm", "yarn"];
            let index = Select::with_theme(&create_theme())
                .with_prompt("Which package manager?")
                .items(&items)
                .default(0)
                .interact_opt()?
                .unwrap_or(0);

            pm_type = String::from(items[index]);
        }
    }

    // If no version, fallback to configuration default
    if pm_version.is_empty() {
        if pm_type == "npm" {
            pm_version = default_npm_version();
        } else if pm_type == "pnpm" {
            pm_version = default_pnpm_version();
        } else if pm_type == "yarn" {
            pm_version = default_yarn_version();
        }
    }

    Ok((pm_type, pm_version))
}

/// Detect the Node.js version from local configuration files,
/// otherwise fallback to the configuration default.
fn detect_node_version(dest_dir: &Path) -> Result<String, AnyError> {
    let nvmrc_path = dest_dir.join(NVMRC.version_filename);

    if nvmrc_path.exists() {
        return Ok(read_to_string(nvmrc_path)?.trim().to_owned());
    }

    let node_version_path = dest_dir.join(NODENV.version_filename);

    if node_version_path.exists() {
        return Ok(read_to_string(node_version_path)?.trim().to_owned());
    }

    Ok(default_node_version())
}

/// Detect potential projects (for existing repos only) by
/// inspecting the `workspaces` field in a root `package.json`.
async fn detect_projects(dest_dir: &Path, yes: bool) -> Result<ProjectsSourceMap, AnyError> {
    let pkg_path = dest_dir.join("package.json");
    let mut projects = HashMap::new();

    if pkg_path.exists() {
        if let Ok(pkg) = PackageJson::load(&pkg_path).await {
            if let Some(workspaces) = pkg.workspaces {
                if yes
                    || Confirm::with_theme(&create_theme())
                        .with_prompt(format!(
                            "Inherit projects from {} workspaces?",
                            color::file("package.json")
                        ))
                        .interact()?
                {
                    let packages = match workspaces {
                        Workspaces::Array(list) => list,
                        Workspaces::Object(object) => object.packages.unwrap_or_default(),
                    };

                    detect_projects_with_globs(dest_dir, packages, &mut projects)?;
                }
            }
        }
    }

    if projects.is_empty() {
        projects.insert("example".to_owned(), "apps/example".to_owned());
    }

    Ok(projects)
}

pub async fn init(dest: &str, yes: bool, force: bool) -> Result<(), AnyError> {
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
    let moon_dir = match verify_dest_dir(&dest_dir, yes, force)? {
        Some(dir) => dir,
        None => return Ok(()),
    };
    let package_manager = detect_package_manager(&dest_dir, yes).await?;
    let node_version = detect_node_version(&dest_dir)?;
    let projects = detect_projects(&dest_dir, yes).await?;

    // Generate a template
    let mut context = Context::new();
    context.insert("package_manager", &package_manager.0);
    context.insert("package_manager_version", &package_manager.1);
    context.insert("node_version", &node_version);
    context.insert("projects", &projects);

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
        &moon_dir.join(CONFIG_PROJECT_FILENAME),
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
.moon/cache"#
    )?;

    println!(
        "Moon has successfully been initialized in {}",
        color::path(&dest_dir),
    );

    Ok(())
}
