use super::InitOptions;
use crate::helpers::fully_qualify_version;
use dialoguer::theme::ColorfulTheme;
use miette::IntoDiagnostic;
use moon_config::load_toolchain_rust_config_template;
use moon_rust_lang::toolchain_toml::ToolchainTomlCache;
use moon_terminal::label_header;
use starbase::AppResult;
use std::path::Path;
use tera::{Context, Tera};

pub fn render_template(context: Context) -> AppResult<String> {
    Tera::one_off(load_toolchain_rust_config_template(), &context, false).into_diagnostic()
}

fn detect_rust_version(dest_dir: &Path) -> AppResult<String> {
    if let Some(toolchain_toml) = ToolchainTomlCache::read(dest_dir)? {
        if let Some(version) = toolchain_toml.toolchain.channel {
            if version == "stable" || version == "beta" || version == "nightly" {
                return Ok(version);
            } else {
                return Ok(fully_qualify_version(&version));
            }
        }
    }

    Ok("1.70.0".into())
}

pub async fn init_rust(
    dest_dir: &Path,
    options: &InitOptions,
    _theme: &ColorfulTheme,
    _parent_context: Option<&mut Context>,
) -> AppResult<String> {
    if !options.yes {
        println!("\n{}\n", label_header("Rust"));
    }

    let rust_version = detect_rust_version(dest_dir)?;

    let mut context = Context::new();
    context.insert("rust_version", &rust_version);
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
