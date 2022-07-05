use std::collections::HashSet;

#[derive(Default)]
pub struct ActionRunnerContext {
    pub passthrough_args: Vec<String>,

    pub primary_targets: HashSet<String>,
}
