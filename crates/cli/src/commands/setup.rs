use crate::helpers::create_progress_bar;
use moon::load_workspace_with_toolchain;
use starbase::AppResult;

pub async fn setup() -> AppResult {
    let done = create_progress_bar("Downloading and installing tools...");

    load_workspace_with_toolchain().await?;

    done("Setup complete", true);

    Ok(())
}
