use super::prompts::prompt_version;
use super::InitOptions;
use crate::helpers::fully_qualify_version;
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Confirm, Select};
use miette::IntoDiagnostic;
use moon_config::load_toolchain_node_config_template;
use moon_console::Console;
use moon_lang::{is_using_dependency_manager, is_using_version_manager};
use moon_node_lang::package_json::PackageJson;
use starbase::AppResult;
use starbase_styles::color;
use starbase_utils::fs;
use std::path::Path;
use tera::{Context, Tera};

pub fn render_template(context: Context) -> AppResult<String> {
    Tera::one_off(load_toolchain_node_config_template(), &context, false).into_diagnostic()
}

/// Detect the Node.js version from local configuration files,
/// otherwise fallback to the configuration default.
fn detect_node_version(dest_dir: &Path) -> AppResult<String> {
    Ok(if is_using_version_manager(dest_dir, ".nvmrc") {
        fully_qualify_version(fs::read_file(dest_dir.join(".nvmrc"))?.trim())
    } else if is_using_version_manager(dest_dir, ".node-version") {
        fully_qualify_version(fs::read_file(dest_dir.join(".node-version"))?.trim())
    } else {
        String::new()
    })
}

fn detect_node_version_manager(dest_dir: &Path) -> AppResult<String> {
    Ok(if is_using_version_manager(dest_dir, ".nvmrc") {
        "nvm".to_owned()
    } else if is_using_version_manager(dest_dir, ".node-version") {
        "nodenv".to_owned()
    } else {
        String::new()
    })
}

/// Verify the package manager to use. If a `package.json` exists,
/// and the `packageManager` field is defined, use that.
fn detect_package_manager(
    dest_dir: &Path,
    options: &InitOptions,
    theme: &ColorfulTheme,
) -> AppResult<(String, String)> {
    let mut pm_type = String::new();
    let mut pm_version = String::new();

    // Extract value from `packageManager` field
    if let Ok(Some(pkg)) = PackageJson::read(dest_dir) {
        if let Some(pm) = pkg.package_manager {
            if pm.contains('@') {
                let mut parts = pm.split('@');

                pm_type = parts.next().unwrap_or_default().to_owned();
                pm_version = parts.next().unwrap_or_default().to_owned();

                // Remove corepack hash
                if let Some(index) = pm_version.find('+') {
                    pm_version = pm_version[0..index].to_owned();
                }
            } else {
                pm_type = pm;
            }
        }
    }

    // If no value, detect based on files
    if pm_type.is_empty() {
        if is_using_dependency_manager(dest_dir, "yarn.lock") {
            pm_type = "yarn".to_owned();
        } else if is_using_dependency_manager(dest_dir, "pnpm-lock.yaml") {
            pm_type = "pnpm".to_owned();
        } else if is_using_dependency_manager(dest_dir, "bun.lockb") {
            pm_type = "bun".to_owned();
        } else if is_using_dependency_manager(dest_dir, "package-lock.json") {
            pm_type = "npm".to_owned();
        }
    }

    // If no value again, ask for explicit input
    if pm_type.is_empty() {
        let items = vec!["npm", "pnpm", "yarn", "bun"];
        let default_index = 0;

        let index = if options.yes || options.minimal {
            default_index
        } else {
            Select::with_theme(theme)
                .with_prompt("Package manager?")
                .items(&items)
                .default(default_index)
                .interact_opt()
                .into_diagnostic()?
                .unwrap_or(default_index)
        };

        pm_type = items[index].to_owned();
    }

    pm_version = prompt_version(&pm_type, options, theme, || Ok(pm_version))?;

    Ok((pm_type, fully_qualify_version(&pm_version)))
}

pub async fn init_node(
    dest_dir: &Path,
    options: &InitOptions,
    theme: &ColorfulTheme,
    console: &Console,
) -> AppResult<String> {
    if !options.yes {
        console.out.print_header("Node")?;

        console.out.write_raw(|buffer| {
            buffer.extend_from_slice(
                format!(
                    "Toolchain: {}\n",
                    color::url("https://moonrepo.dev/docs/concepts/toolchain")
                )
                .as_bytes(),
            );
            buffer.extend_from_slice(
                format!(
                    "Handbook: {}\n",
                    color::url("https://moonrepo.dev/docs/guides/javascript/node-handbook")
                )
                .as_bytes(),
            );
            buffer.extend_from_slice(
                format!(
                    "Config: {}\n\n",
                    color::url("https://moonrepo.dev/docs/config/toolchain#node")
                )
                .as_bytes(),
            );
        })?;

        console.out.flush()?;
    }

    let node_version = prompt_version("Node", options, theme, || detect_node_version(dest_dir))?;
    let node_version_manager = detect_node_version_manager(dest_dir)?;
    let package_manager = detect_package_manager(dest_dir, options, theme)?;

    let infer_tasks = if options.yes || options.minimal {
        false
    } else {
        Confirm::with_theme(theme)
            .with_prompt(format!(
                "Infer {} scripts as moon tasks? {}",
                color::file("package.json"),
                color::muted("(not recommended)")
            ))
            .interact()
            .into_diagnostic()?
    };

    let sync_dependencies = options.yes
        || options.minimal
        || Confirm::with_theme(theme)
            .with_prompt(format!(
                "Sync project relationships as {} {}?",
                color::file("package.json"),
                color::property("dependencies")
            ))
            .interact()
            .into_diagnostic()?;

    let dedupe_lockfile = options.yes
        || options.minimal
        || Confirm::with_theme(theme)
            .with_prompt("Automatically dedupe lockfile when changed?")
            .interact()
            .into_diagnostic()?;

    let mut context = Context::new();
    context.insert("node_version", &node_version);
    context.insert("node_version_manager", &node_version_manager);
    context.insert("package_manager", &package_manager.0);
    context.insert("package_manager_version", &package_manager.1);
    context.insert("infer_tasks", &infer_tasks);
    context.insert("sync_dependencies", &sync_dependencies);
    context.insert("dedupe_lockfile", &dedupe_lockfile);
    context.insert("minimal", &options.minimal);

    render_template(context)
}

#[cfg(test)]
mod tests {
    use super::*;
    use moon_test_utils::assert_snapshot;

    fn create_context() -> Context {
        let mut context = Context::new();
        context.insert("node_version", &"16.0.0");
        context.insert("node_version_manager", &"");
        context.insert("package_manager", &"npm");
        context.insert("package_manager_version", &"8.0.0");
        context.insert("infer_tasks", &false);
        context.insert("dedupe_lockfile", &false);
        context.insert("sync_dependencies", &true);
        context.insert("minimal", &false);
        context
    }

    #[test]
    fn renders_default() {
        assert_snapshot!(render_template(create_context()).unwrap());
    }

    #[test]
    fn renders_minimal() {
        let mut context = create_context();
        context.insert("minimal", &true);

        assert_snapshot!(render_template(context).unwrap());
    }

    #[test]
    fn renders_nvm() {
        let mut context = create_context();
        context.insert("node_version_manager", &"nvm");

        assert_snapshot!(render_template(context).unwrap());
    }

    #[test]
    fn renders_nodenv() {
        let mut context = create_context();
        context.insert("node_version_manager", &"nodenv");

        assert_snapshot!(render_template(context).unwrap());
    }

    #[test]
    fn renders_npm() {
        let mut context = create_context();
        context.insert("package_manager", &"npm");
        context.insert("package_manager_version", &"9.0.0");

        assert_snapshot!(render_template(context).unwrap());
    }

    #[test]
    fn renders_pnpm() {
        let mut context = create_context();
        context.insert("package_manager", &"pnpm");
        context.insert("package_manager_version", &"7.14.0");

        assert_snapshot!(render_template(context).unwrap());
    }

    #[test]
    fn renders_yarn() {
        let mut context = create_context();
        context.insert("package_manager", &"yarn");
        context.insert("package_manager_version", &"3.2.0");

        assert_snapshot!(render_template(context).unwrap());
    }

    #[test]
    fn renders_bun() {
        let mut context = create_context();
        context.insert("package_manager", &"bun");
        context.insert("package_manager_version", &"1.0.0");

        assert_snapshot!(render_template(context).unwrap());
    }

    #[test]
    fn renders_tasks() {
        let mut context = create_context();
        context.insert("infer_tasks", &true);

        assert_snapshot!(render_template(context).unwrap());
    }
}
