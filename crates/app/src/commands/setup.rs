use crate::helpers::create_progress_bar;
use crate::systems::analyze;
use starbase::AppResult;
use tracing::instrument;

#[instrument]
pub async fn setup() -> AppResult {
    let done = create_progress_bar("Downloading and installing tools...");

    analyze::load_toolchain().await?;

    done("Setup complete", true);

    Ok(())
}
