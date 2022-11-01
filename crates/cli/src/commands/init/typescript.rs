use super::InitOptions;
use crate::helpers::AnyError;
use dialoguer::theme::ColorfulTheme;
use dialoguer::Confirm;
use moon_config::load_workspace_typescript_config_template;
use moon_node_lang::tsconfig::TsConfigJson;
use moon_terminal::label_header;
use std::path::Path;
use tera::{Context, Error, Tera};

fn render_template(context: Context) -> Result<String, Error> {
    Tera::one_off(load_workspace_typescript_config_template(), &context, false)
}

pub async fn init_typescript(
    dest_dir: &Path,
    options: &InitOptions,
    theme: &ColorfulTheme,
) -> Result<String, AnyError> {
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
            || Confirm::with_theme(theme)
                .with_prompt("Use project references?")
                .interact()?
    };

    let mut route_cache = false;
    let mut sync_paths = false;

    if project_refs {
        route_cache = options.yes
            || Confirm::with_theme(theme)
                .with_prompt("Route declaration output to moons cache?")
                .interact()?;

        sync_paths = options.yes
            || Confirm::with_theme(theme)
                .with_prompt("Sync project references as path aliases?")
                .interact()?;
    }

    let mut context = Context::new();
    context.insert("project_refs", &project_refs);
    context.insert("route_cache", &route_cache);
    context.insert("sync_paths", &sync_paths);

    Ok(render_template(context)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    #[test]
    fn renders_default() {
        let mut context = Context::new();
        context.insert("project_refs", &false);
        context.insert("route_cache", &false);
        context.insert("sync_paths", &false);

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
