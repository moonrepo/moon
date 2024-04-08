use moon_app_components::{Console, MoonEnv};
use moon_codegen::{templates_command, CodeGenerator};
use moon_workspace::Workspace;
use starbase::system;
use std::sync::Arc;

#[system]
pub async fn templates(
    workspace: ResourceRef<Workspace>,
    console: ResourceRef<Console>,
    moon_env: StateRef<MoonEnv>,
) {
    let generator = CodeGenerator::new(
        &workspace.root,
        &workspace.config.generator,
        Arc::clone(moon_env),
    );

    templates_command(generator, console).await?;
}
