use moon_project::ProjectType;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EnforcerError {
    #[error("Invalid project relationship. Project <id>{0}</id> of type {1} cannot depend on project <id>{2}</id> of type {3}; can only depend on libraries.")]
    InvalidTypeRelationship(String, ProjectType, String, ProjectType),

    #[error("Invalid tag relationship. Project <id>{0}</id> with tag {1} cannot depend on project <id>{2}</id>. The tag {1} requires a dependency to have one of the following tags: {3}.")]
    InvalidTagRelationship(String, String, String, String),
}
