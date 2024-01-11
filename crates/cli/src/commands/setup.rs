use crate::helpers::create_progress_bar;
use moon::load_toolchain;
use starbase::system;

#[system]
pub async fn setup() {
    let done = create_progress_bar("Downloading and installing tools...");

    load_toolchain().await?;

    done("Setup complete", true);
}
