use crate::app::App;
use moon_workspace::Workspace;
use starbase::{Resource, State};

#[derive(State)]
pub struct CurrentCommand(pub App);

#[derive(Resource)]
pub struct WorkspaceInstance(pub Workspace);
