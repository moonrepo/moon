pub mod git;

mod changed_files;
mod process_cache;
mod vcs;

pub use changed_files::*;
pub use vcs::*;

pub type BoxedVcs = Box<dyn Vcs + Send + Sync + 'static>;
