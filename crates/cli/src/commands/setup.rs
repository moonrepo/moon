use indicatif::{ProgressBar, ProgressStyle};
use moon_terminal::create_theme;
use moon_workspace::Workspace;
use moon_action_runner::{ActionRunner, DepGraph};

pub async fn setup() -> Result<(), Box<dyn std::error::Error>> {
    let theme = create_theme();
    let mut workspace = Workspace::load().await?;

    let pb = ProgressBar::new_spinner();
    pb.set_message("Downloading and installing tools...");
    pb.enable_steady_tick(20);

    workspace.toolchain.setup(true).await?;

    pb.set_message("Installing dependencies...");

    ActionRunner::new(workspace).run(DepGraph::for_setup(), None).await?;

    pb.set_style(ProgressStyle::default_spinner().template("{prefix} {msg}"));
    pb.set_prefix(theme.success_prefix.to_string());
    pb.finish_with_message("Setup complete");

    Ok(())
}
