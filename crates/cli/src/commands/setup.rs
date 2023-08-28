use crate::helpers::create_progress_bar;
use moon::load_workspace_with_toolchain;
use starbase::system;

#[system]
pub async fn setup() {
    let done = create_progress_bar("Downloading and installing tools...");

    load_workspace_with_toolchain().await?;

    done("Setup complete", true);
}
