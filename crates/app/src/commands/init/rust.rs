use super::InitOptions;
use super::prompts::*;
use iocraft::prelude::element;
use miette::IntoDiagnostic;
use moon_config::load_toolchain_rust_config_template;
use moon_console::{
    Console,
    ui::{Container, Entry, Section, Style, StyledText},
};
use moon_rust_lang::toolchain_toml::ToolchainTomlCache;
use proto_core::UnresolvedVersionSpec;
use std::path::Path;
use tera::{Context, Tera};
use tracing::instrument;

fn render_template(context: Context) -> miette::Result<String> {
    Tera::one_off(load_toolchain_rust_config_template(), &context, false).into_diagnostic()
}

fn detect_rust_version(dest_dir: &Path) -> miette::Result<Option<UnresolvedVersionSpec>> {
    if let Some(toolchain_toml) = ToolchainTomlCache::read(dest_dir)? {
        if let Some(version) = toolchain_toml.toolchain.channel {
            let rust_version = if version == "stable"
                || version == "beta"
                || version == "nightly"
                || version.starts_with("nightly")
            {
                Some(version)
            } else {
                fully_qualify_version(version)
            };

            return Ok(rust_version.and_then(|v| UnresolvedVersionSpec::parse(v).ok()));
        }
    }

    Ok(None)
}

#[instrument(skip_all)]
pub async fn init_rust(console: &Console, options: &InitOptions) -> miette::Result<String> {
    if !options.yes {
        console.render(element! {
            Container {
                Section(title: "Rust") {
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
                                content: "https://moonrepo.dev/docs/guides/rust/handbook",
                                style: Style::Url
                            )
                        }.into_any()
                    )
                    Entry(
                        name: "Config",
                        value: element! {
                            StyledText(
                                content: "https://moonrepo.dev/docs/config/toolchain#rust",
                                style: Style::Url
                            )
                        }.into_any()
                    )
                }
            }
        })?;
    }

    let rust_version = render_version_prompt(console, options, "Rust", || {
        detect_rust_version(&options.dir)
    })
    .await?;

    let mut context = Context::new();
    if let Some(rust_version) = rust_version {
        context.insert("rust_version", &rust_version);
    }
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
        context.insert("rust_version", &"1.70.0");

        assert_snapshot!(render_template(context).unwrap());
    }

    #[test]
    fn renders_minimal() {
        let mut context = Context::new();
        context.insert("rust_version", &"1.70.0");
        context.insert("minimal", &true);

        assert_snapshot!(render_template(context).unwrap());
    }
}
