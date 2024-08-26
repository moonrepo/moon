use crate::helpers::create_progress_bar;
use crate::session::CliSession;
use crate::systems::analyze;
use starbase::AppResult;
use tracing::instrument;

#[instrument]
pub async fn setup(session: CliSession) -> AppResult {
    let done = create_progress_bar("Downloading and installing tools...");

    analyze::load_toolchain(session.get_toolchain_registry()?).await?;

    done("Setup complete", true);

    Ok(())
}
