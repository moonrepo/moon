use super::InitOptions;
use crate::helpers::AnyError;
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Confirm, Select};
use moon_config::{
    default_node_version, default_npm_version, default_pnpm_version, default_yarn_version,
    load_workspace_node_config_template,
};
use moon_lang::{is_using_package_manager, is_using_version_manager};
use moon_lang_node::package::{PackageJson, PackageWorkspaces};
use moon_lang_node::{NODENV, NPM, NVMRC, PNPM, YARN};
use moon_logger::color;
use moon_project::detect_projects_with_globs;
use moon_terminal::label_header;
use moon_utils::fs;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use tera::{Context, Error, Tera};

fn render_template(context: Context) -> Result<String, Error> {
    Tera::one_off(load_workspace_node_config_template(), &context, false)
}

/// Detect the Node.js version from local configuration files,
/// otherwise fallback to the configuration default.
async fn detect_node_version(dest_dir: &Path) -> Result<(String, String), AnyError> {
    if is_using_version_manager(dest_dir, &NVMRC) {
        return Ok((
            fs::read(dest_dir.join(NVMRC.version_filename))
                .await?
                .trim()
                .to_owned(),
            NVMRC.binary.to_owned(),
        ));
    }

    if is_using_version_manager(dest_dir, &NODENV) {
        return Ok((
            fs::read(dest_dir.join(NODENV.version_filename))
                .await?
                .trim()
                .to_owned(),
            NODENV.binary.to_owned(),
        ));
    }

    Ok((default_node_version(), String::new()))
}

/// Verify the package manager to use. If a `package.json` exists,
/// and the `packageManager` field is defined, use that.
async fn detect_package_manager(
    dest_dir: &Path,
    options: &InitOptions,
    theme: &ColorfulTheme,
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
        let default_index = 0;

        let index = if options.yes {
            default_index
        } else {
            Select::with_theme(theme)
                .with_prompt("Package manager?")
                .items(&items)
                .default(default_index)
                .interact_opt()?
                .unwrap_or(default_index)
        };

        pm_type = items[index].to_owned();
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

// Detect potential projects (for existing repos only) by
// inspecting the `workspaces` field in a root `package.json`.
async fn detect_projects(
    dest_dir: &Path,
    options: &InitOptions,
    parent_context: &mut Context,
    theme: &ColorfulTheme,
) -> Result<(), AnyError> {
    let mut projects = HashMap::new();
    let mut project_globs = vec![];

    if let Ok(Some(pkg)) = PackageJson::read(dest_dir) {
        if let Some(workspaces) = pkg.workspaces {
            let items = vec![
                "Don't inherit",
                "As a list of globs",
                "As a map of project sources",
            ];
            let default_index = 1;

            let index = if options.yes {
                default_index
            } else {
                Select::with_theme(theme)
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
        project_globs.push("apps/*".to_owned());
        project_globs.push("packages/*".to_owned());
    }

    // Sort the projects for template rendering
    let mut sorted_projects = BTreeMap::new();

    for (key, value) in projects {
        sorted_projects.insert(key, value);
    }

    parent_context.insert("projects", &sorted_projects);
    parent_context.insert("project_globs", &project_globs);

    Ok(())
}

pub async fn init_node(
    dest_dir: &Path,
    options: &InitOptions,
    theme: &ColorfulTheme,
    parent_context: Option<&mut Context>,
) -> Result<String, AnyError> {
    if !options.yes {
        println!("\n{}\n", label_header("Node"));
    }

    let node_version = detect_node_version(dest_dir).await?;
    let package_manager = detect_package_manager(dest_dir, options, theme).await?;

    if let Some(parent_context) = parent_context {
        detect_projects(dest_dir, options, parent_context, theme).await?;
    }

    let alias_names = if options.yes {
        false
    } else {
        Confirm::with_theme(theme)
            .with_prompt(format!(
                "Use {} names as moon project aliases?",
                color::file(NPM.manifest_filename)
            ))
            .interact()?
    };

    let infer_tasks = if options.yes {
        false
    } else {
        Confirm::with_theme(theme)
            .with_prompt(format!(
                "Infer {} scripts as moon tasks? {}",
                color::file(NPM.manifest_filename),
                color::muted("(not recommended)")
            ))
            .interact()?
    };

    let mut context = Context::new();
    context.insert("node_version", &node_version.0);
    context.insert("node_version_manager", &node_version.1);
    context.insert("package_manager", &package_manager.0);
    context.insert("package_manager_version", &package_manager.1);
    context.insert("alias_names", &alias_names);
    context.insert("infer_tasks", &infer_tasks);

    Ok(render_template(context)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    #[test]
    fn renders_default() {
        let mut context = Context::new();
        context.insert("node_version", &"16.0.0");
        context.insert("node_version_manager", &"");
        context.insert("package_manager", &"npm");
        context.insert("package_manager_version", &"inherit");
        context.insert("alias_names", &false);
        context.insert("infer_tasks", &false);

        assert_snapshot!(render_template(context).unwrap());
    }

    #[test]
    fn renders_nvm() {
        let mut context = Context::new();
        context.insert("node_version", &"18.1.0");
        context.insert("node_version_manager", &"nvm");
        context.insert("package_manager", &"npm");
        context.insert("package_manager_version", &"inherit");
        context.insert("alias_names", &false);
        context.insert("infer_tasks", &false);

        assert_snapshot!(render_template(context).unwrap());
    }

    #[test]
    fn renders_nodenv() {
        let mut context = Context::new();
        context.insert("node_version", &"18.1.0");
        context.insert("node_version_manager", &"nodenv");
        context.insert("package_manager", &"npm");
        context.insert("package_manager_version", &"inherit");
        context.insert("alias_names", &false);
        context.insert("infer_tasks", &false);

        assert_snapshot!(render_template(context).unwrap());
    }

    #[test]
    fn renders_npm() {
        let mut context = Context::new();
        context.insert("node_version", &"16.0.0");
        context.insert("node_version_manager", &"");
        context.insert("package_manager", &"npm");
        context.insert("package_manager_version", &"9.0.0");
        context.insert("alias_names", &false);
        context.insert("infer_tasks", &false);

        assert_snapshot!(render_template(context).unwrap());
    }

    #[test]
    fn renders_pnpm() {
        let mut context = Context::new();
        context.insert("node_version", &"16.0.0");
        context.insert("node_version_manager", &"");
        context.insert("package_manager", &"pnpm");
        context.insert("package_manager_version", &"7.14.0");
        context.insert("alias_names", &false);
        context.insert("infer_tasks", &false);

        assert_snapshot!(render_template(context).unwrap());
    }

    #[test]
    fn renders_yarn() {
        let mut context = Context::new();
        context.insert("node_version", &"16.0.0");
        context.insert("node_version_manager", &"");
        context.insert("package_manager", &"yarn");
        context.insert("package_manager_version", &"3.2.0");
        context.insert("alias_names", &false);
        context.insert("infer_tasks", &false);

        assert_snapshot!(render_template(context).unwrap());
    }

    #[test]
    fn renders_alias_and_tasks() {
        let mut context = Context::new();
        context.insert("node_version", &"16.0.0");
        context.insert("node_version_manager", &"");
        context.insert("package_manager", &"npm");
        context.insert("package_manager_version", &"inherit");
        context.insert("alias_names", &true);
        context.insert("infer_tasks", &true);

        assert_snapshot!(render_template(context).unwrap());
    }
}
