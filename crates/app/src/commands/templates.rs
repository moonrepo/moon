use crate::session::CliSession;
use moon_codegen::{CodeGenerator, TemplatesArgs, templates_command};
use starbase::AppResult;
use std::sync::Arc;
use tracing::instrument;

#[instrument(skip_all)]
pub async fn templates(session: CliSession, args: TemplatesArgs) -> AppResult {
    let generator = CodeGenerator::new(
        &session.workspace_root,
        &session.workspace_config.generator,
        Arc::clone(&session.moon_env),
    );

    templates_command(generator, &session.console, &args).await
}
