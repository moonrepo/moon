use clap::ValueEnum;
use std::collections::HashSet;

#[derive(ValueEnum, Clone, Debug)]
pub enum ProfileType {
    Cpu,
    Heap,
}

#[derive(Default)]
pub struct ActionRunnerContext {
    pub passthrough_args: Vec<String>,

    pub primary_targets: HashSet<String>,

    pub profile: Option<ProfileType>,
}
