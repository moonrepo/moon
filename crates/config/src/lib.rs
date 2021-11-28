mod errors;
mod validators;

pub mod constants;
pub mod global_project;
pub mod workspace;

// Re-exports structs for convenience
pub use validator::ValidationErrors;
pub use workspace::WorkspaceConfig;
