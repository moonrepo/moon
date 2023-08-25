use crate::app::App;
use moon_workspace::Workspace;
use starbase::State;

#[derive(State)]
pub struct CurrentCommand(pub App);

#[derive(State)]
pub struct WorkspaceInstance(pub Workspace);
