use crate::session::CliSession;
use moon_codegen::{templates_command, CodeGenerator};
use starbase::AppResult;
use std::sync::Arc;
use tracing::instrument;

#[instrument(skip_all)]
pub async fn templates(session: CliSession) -> AppResult {
    let generator = CodeGenerator::new(
        &session.workspace_root,
        &session.workspace_config.generator,
        Arc::clone(&session.moon_env),
    );

    templates_command(generator, &session.console).await?;

    Ok(())
}
