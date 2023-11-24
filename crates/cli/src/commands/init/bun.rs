use super::InitOptions;
use dialoguer::theme::ColorfulTheme;
use miette::IntoDiagnostic;
use moon_config::load_toolchain_bun_config_template;
use moon_terminal::label_header;
use starbase::AppResult;
use std::path::Path;
use tera::{Context, Tera};

pub fn render_template(context: Context) -> AppResult<String> {
    Tera::one_off(load_toolchain_bun_config_template(), &context, false).into_diagnostic()
}

pub async fn init_bun(
    _dest_dir: &Path,
    options: &InitOptions,
    _theme: &ColorfulTheme,
) -> AppResult<String> {
    if !options.yes {
        println!("\n{}\n", label_header("Bun"));
    }

    let mut context = Context::new();
    context.insert("bun_version", "1.0.0");
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

        assert_snapshot!(render_template(context).unwrap());
    }

    #[test]
    fn renders_minimal() {
        let mut context = Context::new();
        context.insert("bun_version", &"1.0.0");
        context.insert("minimal", &true);

        assert_snapshot!(render_template(context).unwrap());
    }
}
