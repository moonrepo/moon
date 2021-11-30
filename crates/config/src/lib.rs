mod errors;
mod validators;

pub mod constants;
pub mod global_project;
pub mod project;
pub mod workspace;

// Re-exports structs for convenience
pub use global_project::GlobalProjectConfig;
pub use project::ProjectConfig;
pub use validator::ValidationErrors;
pub use workspace::WorkspaceConfig;
