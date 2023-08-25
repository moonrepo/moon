use crate::app::App;
use starbase::State;

#[derive(State)]
pub struct CurrentCommand(pub App);
