use crate::errors::PipelineError;
use moon_action::{Action, ActionNode};
use moon_logger::{color, debug, error, trace};

pub async fn process_action(action: &mut Action) -> Result<(), PipelineError> {
    trace!(
        target: &action.log_target,
        "Running action {}",
        color::muted_light(&action.label)
    );

    match action.node.as_ref().unwrap() {
        ActionNode::SetupTool(runtime) => {}
        ActionNode::InstallDeps(runtime) => {}
        ActionNode::InstallProjectDeps(runtime, project_id) => {}
        ActionNode::SyncProject(runtime, project_id) => {}
        ActionNode::RunTarget(target_id) => {}
    };

    Ok(())
}
