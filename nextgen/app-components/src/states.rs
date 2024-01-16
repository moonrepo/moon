use moon_env::MoonEnvironment;
use proto_core::ProtoEnvironment;
use starbase::State;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(State)]
pub struct MoonEnv(pub Arc<MoonEnvironment>);

#[derive(State)]
pub struct ProtoEnv(pub Arc<ProtoEnvironment>);

#[derive(Debug, State)]
pub struct WorkingDir(pub PathBuf);

#[derive(Debug, State)]
pub struct WorkspaceRoot(pub PathBuf);
