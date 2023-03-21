mod errors;

pub use errors::*;
use moon_project::{Project, ProjectType};

pub fn enforce_project_type_relationships(
    source: &Project,
    dependency: &Project,
) -> Result<(), EnforcerError> {
    let enforce_is_library = |project: &Project| -> Result<(), EnforcerError> {
        if matches!(project.type_of, ProjectType::Library) {
            return Ok(());
        }

        Err(EnforcerError::InvalidTypeRelationship(
            source.id.clone(),
            source.type_of,
            project.id.clone(),
            project.type_of,
        ))
    };

    match source.type_of {
        ProjectType::Application | ProjectType::Library | ProjectType::Tool => {
            enforce_is_library(dependency)?;
        }
        ProjectType::Unknown => {
            // Do nothing?
        }
    };

    Ok(())
}

pub fn enforce_tag_relationships(
    source: &Project,
    source_tag: &String,
    dependency: &Project,
    dependency_tags: &[String],
) -> Result<(), EnforcerError> {
    // Source project isn't using the source tag
    if source_tag.is_empty()
        || source.config.tags.is_empty()
        || !source.config.tags.contains(source_tag)
    {
        return Ok(());
    }

    // Dependency project doesn't have any tags
    if dependency_tags.is_empty() || dependency.config.tags.is_empty() {
        return Ok(());
    }

    // Dependency has the tag!
    if dependency.config.tags.contains(source_tag) {
        return Ok(());
    }

    Err(EnforcerError::InvalidTagRelationship(
        source.id.clone(),
        source_tag.clone(),
        dependency.id.clone(),
        dependency_tags.join(", "),
    ))
}
