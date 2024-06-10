use super::InitOptions;
use dialoguer::theme::ColorfulTheme;
use dialoguer::Confirm;
use miette::IntoDiagnostic;
use moon_config::load_toolchain_typescript_config_template;
use moon_console::Console;
use moon_typescript_lang::TsConfigJsonCache;
use starbase::AppResult;
use starbase_styles::color;
use std::path::Path;
use tera::{Context, Tera};
use tracing::instrument;

pub fn render_template(context: Context) -> AppResult<String> {
    Tera::one_off(load_toolchain_typescript_config_template(), &context, false).into_diagnostic()
}

#[instrument(skip_all)]
pub async fn init_typescript(
    dest_dir: &Path,
    options: &InitOptions,
    theme: &ColorfulTheme,
    console: &Console,
) -> AppResult<String> {
    if !options.yes {
        console.out.print_header("TypeScript")?;

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
                    color::url(
                        "https://moonrepo.dev/docs/guides/javascript/typescript-project-refs"
                    )
                )
                .as_bytes(),
            );
            buffer.extend_from_slice(
                format!(
                    "Config: {}\n\n",
                    color::url("https://moonrepo.dev/docs/config/toolchain#typescript")
                )
                .as_bytes(),
            );
        })?;

        console.out.flush()?;
    }

    let project_refs = if let Ok(Some(tsconfig)) = TsConfigJsonCache::read(dest_dir) {
        tsconfig
            .data
            .compiler_options
            .and_then(|o| o.composite)
            .unwrap_or(tsconfig.data.references.is_some())
    } else {
        options.yes
            || options.minimal
            || Confirm::with_theme(theme)
                .with_prompt(format!("Use project {}?", color::property("references")))
                .interact()
                .into_diagnostic()?
    };

    let mut route_cache = false;
    let mut sync_paths = false;
    let mut include_refs = false;

    if project_refs && !options.minimal {
        route_cache = options.yes
            || Confirm::with_theme(theme)
                .with_prompt("Route declaration output to moon's cache?")
                .interact()
                .into_diagnostic()?;

        sync_paths = options.yes
            || Confirm::with_theme(theme)
                .with_prompt(format!(
                    "Sync project references as {} aliases?",
                    color::property("paths")
                ))
                .interact()
                .into_diagnostic()?;

        include_refs = options.yes
            || Confirm::with_theme(theme)
                .with_prompt(format!(
                    "Append project reference sources to {}?",
                    color::property("include")
                ))
                .interact()
                .into_diagnostic()?;
    }

    let include_shared = if options.yes || options.minimal {
        false
    } else {
        Confirm::with_theme(theme)
            .with_prompt(format!(
                "Append shared types to {}?",
                color::property("include")
            ))
            .interact()
            .into_diagnostic()?
    };

    let mut context = Context::new();
    context.insert("project_refs", &project_refs);
    context.insert("route_cache", &route_cache);
    context.insert("sync_paths", &sync_paths);
    context.insert("include_project_refs", &include_refs);
    context.insert("include_shared_types", &include_shared);
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
        context.insert("project_refs", &false);
        context.insert("route_cache", &false);
        context.insert("sync_paths", &false);
        context.insert("include_project_refs", &false);
        context.insert("include_shared_types", &false);

        assert_snapshot!(render_template(context).unwrap());
    }

    #[test]
    fn renders_minimal() {
        let mut context = Context::new();
        context.insert("project_refs", &false);
        context.insert("route_cache", &false);
        context.insert("sync_paths", &false);
        context.insert("minimal", &true);
        context.insert("include_project_refs", &false);
        context.insert("include_shared_types", &false);

        assert_snapshot!(render_template(context).unwrap());
    }

    #[test]
    fn renders_project_refs() {
        let mut context = Context::new();
        context.insert("project_refs", &true);
        context.insert("route_cache", &false);
        context.insert("sync_paths", &false);
        context.insert("include_project_refs", &false);
        context.insert("include_shared_types", &false);

        assert_snapshot!(render_template(context).unwrap());
    }

    #[test]
    fn renders_route_cache() {
        let mut context = Context::new();
        context.insert("project_refs", &true);
        context.insert("route_cache", &true);
        context.insert("sync_paths", &false);
        context.insert("include_project_refs", &false);
        context.insert("include_shared_types", &false);

        assert_snapshot!(render_template(context).unwrap());
    }

    #[test]
    fn renders_sync_paths() {
        let mut context = Context::new();
        context.insert("project_refs", &true);
        context.insert("route_cache", &true);
        context.insert("sync_paths", &true);
        context.insert("include_project_refs", &false);
        context.insert("include_shared_types", &false);

        assert_snapshot!(render_template(context).unwrap());
    }
}
