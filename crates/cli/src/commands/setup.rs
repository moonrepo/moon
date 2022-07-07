use indicatif::ProgressBar;
use moon_workspace::Workspace;

pub async fn setup() -> Result<(), Box<dyn std::error::Error>> {
    let pb = ProgressBar::new_spinner();
    pb.set_message("Downloading and installing tools...");
    pb.enable_steady_tick(20);

    let mut workspace = Workspace::load().await?;

    workspace.toolchain.setup(true).await?;

    pb.finish_with_message("Installation complete");
    Ok(())
}
