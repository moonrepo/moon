use super::{append_workspace_config, InitOptions};
use crate::helpers::AnyError;
use dialoguer::theme::ColorfulTheme;
use dialoguer::Confirm;
use moon_config::load_workspace_typescript_config_template;
use moon_lang_node::tsconfig::TsConfigJson;
use std::path::Path;
use tera::{Context, Tera};

pub async fn init_typescript(
    dest_dir: &Path,
    options: &InitOptions,
    theme: &ColorfulTheme,
) -> Result<(), AnyError> {
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

    append_workspace_config(
        dest_dir,
        Tera::one_off(load_workspace_typescript_config_template(), &context, false)?,
    )?;

    Ok(())
}
