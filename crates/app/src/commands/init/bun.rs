use super::InitOptions;
use super::prompts::*;
use iocraft::prelude::element;
use miette::IntoDiagnostic;
use moon_config::load_toolchain_bun_config_template;
use moon_console::{
    Console,
    ui::{Container, Entry, Section, Style, StyledText},
};
use moon_pdk_api::{PromptType, SettingPrompt};
use tera::{Context, Tera};
use tracing::instrument;

pub fn render_template(context: Context) -> miette::Result<String> {
    Tera::one_off(load_toolchain_bun_config_template(), &context, false).into_diagnostic()
}

#[instrument(skip_all)]
pub async fn init_bun(console: &Console, options: &InitOptions) -> miette::Result<String> {
    if !options.yes {
        console.render(element! {
            Container {
                Section(title: "Bun") {
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
                                content: "https://moonrepo.dev/docs/guides/javascript/bun-handbook",
                                style: Style::Url
                            )
                        }.into_any()
                    )
                    Entry(
                        name: "Config",
                        value: element! {
                            StyledText(
                                content: "https://moonrepo.dev/docs/config/toolchain#bun",
                                style: Style::Url
                            )
                        }.into_any()
                    )
                }
            }
        })?;
    }

    let bun_version = render_version_prompt(console, options, "Bun", || Ok(None)).await?;

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

    let mut context = Context::new();
    if let Some(bun_version) = bun_version {
        context.insert("bun_version", &bun_version);
    }
    context.insert("sync_dependencies", &sync_dependencies);
    context.insert("minimal", &options.minimal);

    render_template(context)
}

#[cfg(test)]
mod tests {
    use super::*;
    use starbase_sandbox::assert_snapshot;

    #[test]
    fn renders_default() {
        let mut context = Context::new();
        context.insert("bun_version", &"1.0.0");
        context.insert("infer_tasks", &false);
        context.insert("sync_dependencies", &true);

        assert_snapshot!(render_template(context).unwrap());
    }

    #[test]
    fn renders_minimal() {
        let mut context = Context::new();
        context.insert("bun_version", &"1.0.0");
        context.insert("sync_dependencies", &true);
        context.insert("minimal", &true);

        assert_snapshot!(render_template(context).unwrap());
    }
}
