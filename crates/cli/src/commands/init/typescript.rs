use super::InitOptions;
use dialoguer::theme::ColorfulTheme;
use dialoguer::Confirm;
use miette::IntoDiagnostic;
use moon_config::load_toolchain_typescript_config_template;
use moon_terminal::label_header;
use moon_typescript_lang::TsConfigJson;
use starbase::AppResult;
use std::path::Path;
use tera::{Context, Tera};

pub fn render_template(context: Context) -> AppResult<String> {
    Tera::one_off(load_toolchain_typescript_config_template(), &context, false).into_diagnostic()
}

pub async fn init_typescript(
    dest_dir: &Path,
    options: &InitOptions,
    theme: &ColorfulTheme,
) -> AppResult<String> {
    if !options.yes {
        println!("\n{}\n", label_header("TypeScript"));
    }

    let project_refs = if let Ok(Some(tsconfig)) = TsConfigJson::read(dest_dir) {
        match tsconfig.compiler_options {
            Some(co) => co.composite.unwrap_or_default(),
            None => tsconfig.references.is_some(),
        }
    } else {
        options.yes
            || options.minimal
            || Confirm::with_theme(theme)
                .with_prompt("Use project references?")
                .interact()
                .into_diagnostic()?
    };

    let mut route_cache = false;
    let mut sync_paths = false;

    if project_refs && !options.minimal {
        route_cache = options.yes
            || Confirm::with_theme(theme)
                .with_prompt("Route declaration output to moons cache?")
                .interact()
                .into_diagnostic()?;

        sync_paths = options.yes
            || Confirm::with_theme(theme)
                .with_prompt("Sync project references as path aliases?")
                .interact()
                .into_diagnostic()?;
    }

    let mut context = Context::new();
    context.insert("project_refs", &project_refs);
    context.insert("route_cache", &route_cache);
    context.insert("sync_paths", &sync_paths);
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
        context.insert("project_refs", &false);
        context.insert("route_cache", &false);
        context.insert("sync_paths", &false);

        assert_snapshot!(render_template(context).unwrap());
    }

    #[test]
    fn renders_minimal() {
        let mut context = Context::new();
        context.insert("project_refs", &false);
        context.insert("route_cache", &false);
        context.insert("sync_paths", &false);
        context.insert("minimal", &true);

        assert_snapshot!(render_template(context).unwrap());
    }

    #[test]
    fn renders_project_refs() {
        let mut context = Context::new();
        context.insert("project_refs", &true);
        context.insert("route_cache", &false);
        context.insert("sync_paths", &false);

        assert_snapshot!(render_template(context).unwrap());
    }

    #[test]
    fn renders_route_cache() {
        let mut context = Context::new();
        context.insert("project_refs", &true);
        context.insert("route_cache", &true);
        context.insert("sync_paths", &false);

        assert_snapshot!(render_template(context).unwrap());
    }

    #[test]
    fn renders_sync_paths() {
        let mut context = Context::new();
        context.insert("project_refs", &true);
        context.insert("route_cache", &true);
        context.insert("sync_paths", &true);

        assert_snapshot!(render_template(context).unwrap());
    }
}