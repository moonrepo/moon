use indicatif::ProgressBar;
use moon_workspace::Workspace;

pub async fn teardown() -> Result<(), Box<dyn std::error::Error>> {
    let pb = ProgressBar::new_spinner();
    pb.set_message("Tearing down toolchain and uninstalling tools...");
    pb.enable_steady_tick(20);

    let mut workspace = Workspace::load().await?;

    workspace.toolchain.teardown().await?;
    pb.finish_with_message("Uninstalling complete");
    Ok(())
}
