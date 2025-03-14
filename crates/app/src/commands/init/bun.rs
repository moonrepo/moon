use super::InitOptions;
use super::prompts::prompt_version;
use dialoguer::Confirm;
use dialoguer::theme::ColorfulTheme;
use miette::IntoDiagnostic;
use moon_config::load_toolchain_bun_config_template;
use moon_console::MoonConsole;
use starbase_styles::color;
use tera::{Context, Tera};
use tracing::instrument;

pub fn render_template(context: Context) -> miette::Result<String> {
    Tera::one_off(load_toolchain_bun_config_template(), &context, false).into_diagnostic()
}

#[instrument(skip_all)]
pub async fn init_bun(
    options: &InitOptions,
    theme: &ColorfulTheme,
    console: &MoonConsole,
) -> miette::Result<String> {
    if !options.yes {
        console.print_header("Bun")?;

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
                    color::url("https://moonrepo.dev/docs/guides/javascript/bun-handbook")
                )
                .as_bytes(),
            );
            buffer.extend_from_slice(
                format!(
                    "Config: {}\n\n",
                    color::url("https://moonrepo.dev/docs/config/toolchain#bun")
                )
                .as_bytes(),
            );

            Ok(())
        })?;

        console.out.flush()?;
    }

    let bun_version = prompt_version("Bun", options, theme, || Ok(String::new()))?;

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

    let mut context = Context::new();
    context.insert("bun_version", &bun_version);
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
