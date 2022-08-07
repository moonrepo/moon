use indicatif::{ProgressBar, ProgressStyle};
use moon_terminal::create_theme;
use moon_workspace::Workspace;
use std::time::Duration;

pub async fn setup() -> Result<(), Box<dyn std::error::Error>> {
    let theme = create_theme();

    let pb = ProgressBar::new_spinner();
    pb.set_message("Downloading and installing tools...");
    pb.enable_steady_tick(Duration::from_millis(50));

    let mut workspace = Workspace::load().await?;

    workspace.toolchain.setup(true).await?;

    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{prefix} {msg}")
            .unwrap(),
    );
    pb.set_prefix(theme.success_prefix.to_string());
    pb.finish_with_message("Setup complete");

    Ok(())
}
