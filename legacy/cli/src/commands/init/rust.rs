use super::prompts::prompt_version;
use super::InitOptions;
use crate::helpers::fully_qualify_version;
use dialoguer::theme::ColorfulTheme;
use miette::IntoDiagnostic;
use moon_config::load_toolchain_rust_config_template;
use moon_console::Console;
use moon_rust_lang::toolchain_toml::ToolchainTomlCache;
use starbase::AppResult;
use starbase_styles::color;
use std::path::Path;
use tera::{Context, Tera};

fn render_template(context: Context) -> AppResult<String> {
    Tera::one_off(load_toolchain_rust_config_template(), &context, false).into_diagnostic()
}

fn detect_rust_version(dest_dir: &Path) -> AppResult<String> {
    if let Some(toolchain_toml) = ToolchainTomlCache::read(dest_dir)? {
        if let Some(version) = toolchain_toml.toolchain.channel {
            let rust_version = if version == "stable"
                || version == "beta"
                || version == "nightly"
                || version.starts_with("nightly")
            {
                version
            } else {
                fully_qualify_version(&version)
            };

            return Ok(rust_version);
        }
    }

    Ok(String::new())
}

pub async fn init_rust(
    dest_dir: &Path,
    options: &InitOptions,
    theme: &ColorfulTheme,
    console: &Console,
) -> AppResult<String> {
    if !options.yes {
        console.out.print_header("Rust")?;

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
                    color::url("https://moonrepo.dev/docs/guides/rust/handbook")
                )
                .as_bytes(),
            );
            buffer.extend_from_slice(
                format!(
                    "Config: {}\n\n",
                    color::url("https://moonrepo.dev/docs/config/toolchain#rust")
                )
                .as_bytes(),
            );
        })?;

        console.out.flush()?;
    }

    let rust_version = prompt_version("Rust", options, theme, || detect_rust_version(dest_dir))?;

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
