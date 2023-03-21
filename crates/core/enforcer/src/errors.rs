use moon_project::ProjectType;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EnforcerError {
    #[error("Invalid project relationship. Project {0} of type {1} cannot depend on project {2} of type {3}. Projects can only depend on libraries.")]
    InvalidTypeRelationship(String, ProjectType, String, ProjectType),

    #[error("Invalid tag relationship. Project {0} with tag {1} cannot depend on project {2}. The tag {1} requires a dependency to have one of the following tags: {3}.")]
    InvalidTagRelationship(String, String, String, String),
}
