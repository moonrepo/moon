mod git;
mod git_submodule;
pub mod gitx;

mod git_worktree;
mod jj;
mod jj_workspace;
mod process_cache;
mod touched_files;
mod vcs;

pub use git::*;
pub use git_worktree::*;
pub use jj::*;
pub use jj_workspace::*;
pub use touched_files::*;
pub use vcs::*;

pub type BoxedVcs = Box<dyn Vcs + Send + Sync + 'static>;
