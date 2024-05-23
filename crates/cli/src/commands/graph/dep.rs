use super::action::*;
use moon_common::color;
use moon_workspace::Workspace;
use starbase::system;
use tracing::warn;

#[system]
pub async fn dep_graph(args: ArgsRef<ActionGraphArgs>, workspace: ResourceMut<Workspace>) {
    warn!(
        "This command is deprecated. Use {} instead.",
        color::shell("moon action-graph")
    );

    internal_action_graph(&args, workspace).await?;
}
