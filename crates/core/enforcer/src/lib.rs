mod errors;

pub use errors::*;
use moon_project::{Project, ProjectType};

pub fn enforce_project_type_relationships(
    source: &Project,
    dependency: &Project,
) -> Result<(), EnforcerError> {
    let valid = match source.type_of {
        ProjectType::Application => {
            matches!(dependency.type_of, ProjectType::Library | ProjectType::Tool)
        }
        ProjectType::Library | ProjectType::Tool => {
            matches!(dependency.type_of, ProjectType::Library)
        }
        ProjectType::Unknown => {
            // Do nothing?
            true
        }
    };

    if !valid {
        return Err(EnforcerError::InvalidTypeRelationship(
            source.id.clone(),
            source.type_of,
            dependency.id.clone(),
            dependency.type_of,
        ));
    }

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
