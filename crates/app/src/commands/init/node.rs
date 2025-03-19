use super::InitOptions;
use super::prompts::*;
use iocraft::prelude::element;
use miette::IntoDiagnostic;
use moon_config::load_toolchain_node_config_template;
use moon_console::{
    Console,
    ui::{Container, Entry, Section, Style, StyledText},
};
use moon_lang::{is_using_dependency_manager, is_using_version_manager};
use moon_node_lang::PackageJsonCache;
use moon_pdk_api::{PromptType, SettingPrompt};
use proto_core::UnresolvedVersionSpec;
use starbase_utils::fs;
use starbase_utils::json::JsonValue;
use tera::{Context, Tera};
use tracing::instrument;

pub fn render_template(context: Context) -> miette::Result<String> {
    Tera::one_off(load_toolchain_node_config_template(), &context, false).into_diagnostic()
}

fn detect_node_version(options: &InitOptions) -> miette::Result<Option<UnresolvedVersionSpec>> {
    Ok(if is_using_version_manager(&options.dir, ".nvmrc") {
        UnresolvedVersionSpec::parse(fs::read_file(options.dir.join(".nvmrc"))?).ok()
    } else if is_using_version_manager(&options.dir, ".node-version") {
        UnresolvedVersionSpec::parse(fs::read_file(options.dir.join(".node-version"))?).ok()
    } else {
        None
    })
}

fn detect_node_version_manager(options: &InitOptions) -> miette::Result<String> {
    Ok(if is_using_version_manager(&options.dir, ".nvmrc") {
        "nvm".to_owned()
    } else if is_using_version_manager(&options.dir, ".node-version") {
        "nodenv".to_owned()
    } else {
        String::new()
    })
}

async fn detect_package_manager(
    console: &Console,
    options: &InitOptions,
) -> miette::Result<(String, Option<UnresolvedVersionSpec>)> {
    let mut pm_type = String::new();
    let mut pm_version = String::new();

    // Extract value from `packageManager` field
    if let Ok(Some(pkg)) = PackageJsonCache::read(&options.dir) {
        if let Some(pm) = pkg.data.package_manager {
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
        if is_using_dependency_manager(&options.dir, "yarn.lock") {
            pm_type = "yarn".to_owned();
        } else if is_using_dependency_manager(&options.dir, "pnpm-lock.yaml") {
            pm_type = "pnpm".to_owned();
        } else if is_using_dependency_manager(&options.dir, "bun.lockb") {
            pm_type = "bun".to_owned();
        } else if is_using_dependency_manager(&options.dir, "package-lock.json") {
            pm_type = "npm".to_owned();
        }
    }

    // If no value again, ask for explicit input
    if pm_type.is_empty() {
        let pm = render_prompt(
            console,
            options,
            &SettingPrompt::new(
                "packageManager",
                "Package manager?",
                PromptType::Select {
                    default_index: 0,
                    options: vec!["npm".into(), "pnpm".into(), "yarn".into(), "bun".into()],
                },
            ),
        )
        .await?;

        if let Some(JsonValue::String(inner)) = pm {
            pm_type = inner;
        }
    }

    let pm_version = render_version_prompt(console, options, &pm_type, || {
        if pm_version.is_empty() {
            Ok(None)
        } else {
            Ok(UnresolvedVersionSpec::parse(&pm_version).ok())
        }
    })
    .await?;

    Ok((pm_type, pm_version))
}

#[instrument(skip_all)]
pub async fn init_node(console: &Console, options: &InitOptions) -> miette::Result<String> {
    if !options.yes {
        console.render(element! {
            Container {
                Section(title: "Node") {
                    Entry(
                        name: "Toolchain",
                        value: element! {
                            StyledText(
                                content: "https://moonrepo.dev/docs/concepts/toolchain",
                                style: Style::Url
                            )
                        }.into_any()
                    )
                    Entry(
                        name: "Handbook",
                        value: element! {
                            StyledText(
                                content: "https://moonrepo.dev/docs/guides/javascript/node-handbook",
                                style: Style::Url
                            )
                        }.into_any()
                    )
                    Entry(
                        name: "Config",
                        value: element! {
                            StyledText(
                                content: "https://moonrepo.dev/docs/config/toolchain#node",
                                style: Style::Url
                            )
                        }.into_any()
                    )
                }
            }
        })?;
    }

    let node_version =
        render_version_prompt(console, options, "Node", || detect_node_version(options)).await?;
    let node_version_manager = detect_node_version_manager(options)?;
    let package_manager = detect_package_manager(console, options).await?;

    let infer_tasks = render_prompt(
        console,
        options,
        &SettingPrompt::new(
            "inferTasks",
            "Infer <file>package.json</file> scripts as moon tasks? <muted>(not recommended)</muted>",
            PromptType::Confirm { default: false },
        ),
    )
    .await?;

    let sync_dependencies = render_prompt(
        console,
        options,
        &SettingPrompt::new(
            "syncDependencies",
            "Sync project relationships as <file>package.json</file> <property>dependencies</property>?",
            PromptType::Confirm { default: true },
        ),
    )
    .await?;

    let dedupe_lockfile = render_prompt(
        console,
        options,
        &SettingPrompt::new(
            "dedupeLockfile",
            "Automatically dedupe lockfile when changed?",
            PromptType::Confirm { default: true },
        ),
    )
    .await?;

    let mut context = Context::new();
    if let Some(node_version) = node_version {
        context.insert("node_version", &node_version);
    }
    context.insert("node_version_manager", &node_version_manager);
    context.insert("package_manager", &package_manager.0);
    if let Some(package_manager_version) = package_manager.1 {
        context.insert("package_manager_version", &package_manager_version);
    }
    context.insert("infer_tasks", &infer_tasks);
    context.insert("sync_dependencies", &sync_dependencies);
    context.insert("dedupe_lockfile", &dedupe_lockfile);
    context.insert("minimal", &options.minimal);

    render_template(context)
}

#[cfg(test)]
mod tests {
    use super::*;
    use starbase_sandbox::assert_snapshot;

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
