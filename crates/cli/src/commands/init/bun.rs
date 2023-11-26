use super::prompts::prompt_version;
use super::InitOptions;
use dialoguer::theme::ColorfulTheme;
use dialoguer::Confirm;
use miette::IntoDiagnostic;
use moon_bun_lang::BUNPM;
use moon_config::load_toolchain_bun_config_template;
use moon_terminal::label_header;
use starbase::AppResult;
use starbase_styles::color;
use std::path::Path;
use tera::{Context, Tera};

pub fn render_template(context: Context) -> AppResult<String> {
    Tera::one_off(load_toolchain_bun_config_template(), &context, false).into_diagnostic()
}

pub async fn init_bun(
    _dest_dir: &Path,
    options: &InitOptions,
    theme: &ColorfulTheme,
) -> AppResult<String> {
    if !options.yes {
        println!("\n{}\n", label_header("Bun"));

        println!(
            "Toolchain: {}",
            color::url("https://moonrepo.dev/docs/concepts/toolchain")
        );
        println!(
            "Handbook: {}",
            color::url("https://moonrepo.dev/docs/guides/javascript/bun-handbook")
        );
        println!(
            "Config: {}\n",
            color::url("https://moonrepo.dev/docs/config/toolchain#bun")
        );
    }

    let bun_version = prompt_version("Bun", options, theme, || Ok(String::new()))?;

    let sync_dependencies = options.yes
        || options.minimal
        || Confirm::with_theme(theme)
            .with_prompt(format!(
                "Sync project relationships as {} {}?",
                color::file(BUNPM.manifest),
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
    use moon_test_utils::assert_snapshot;

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
