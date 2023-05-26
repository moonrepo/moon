use miette::Diagnostic;
use moon_project::ProjectType;
use starbase_styles::{Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum EnforcerError {
    #[error("Invalid project relationship. Project {} of type {1} cannot depend on project {} of type {3}; can only depend on libraries.", .0.style(Style::Id), .2.style(Style::Id))]
    InvalidTypeRelationship(String, ProjectType, String, ProjectType),

    #[error("Invalid tag relationship. Project {} with tag {1} cannot depend on project {}. The tag {1} requires a dependency to have one of the following tags: {3}.", .0.style(Style::Id), .2.style(Style::Id))]
    InvalidTagRelationship(String, String, String, String),
}
