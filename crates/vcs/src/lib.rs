mod git;
mod process_cache;
mod touched_files;
mod vcs;

pub use git::*;
pub use touched_files::*;
pub use vcs::*;

pub type BoxedVcs = Box<dyn Vcs + Send + Sync + 'static>;
