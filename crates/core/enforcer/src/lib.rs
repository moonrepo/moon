mod errors;

pub use errors::*;
use moon_project::{Project, ProjectType};

pub fn enforce_project_type_relationships(
    source: &Project,
    dependency: &Project,
) -> Result<(), EnforcerError> {
    let valid = match source.type_of {
        ProjectType::Application => {
            matches!(
                dependency.type_of,
                ProjectType::Library | ProjectType::Tool | ProjectType::Unknown
            )
        }
        ProjectType::Library | ProjectType::Tool => {
            matches!(
                dependency.type_of,
                ProjectType::Library | ProjectType::Unknown
            )
        }
        ProjectType::Unknown => {
            // Do nothing?
            true
        }
    };

    if !valid {
        return Err(EnforcerError::InvalidTypeRelationship(
            source.id.to_string(),
            source.type_of,
            dependency.id.to_string(),
            dependency.type_of,
        ));
    }

    Ok(())
}

pub fn enforce_tag_relationships(
    source: &Project,
    source_tag: &String,
    dependency: &Project,
    required_tags: &[String],
) -> Result<(), EnforcerError> {
    // Source project isn't using the source tag
    if source_tag.is_empty()
        || source.config.tags.is_empty()
        || !source.config.tags.contains(source_tag)
    {
        return Ok(());
    }

    // Dependency project doesn't have any tags
    if required_tags.is_empty() {
        return Ok(());
    }

    // Dependency has the source tag or one of the allowed tags
    if dependency.config.tags.contains(source_tag)
        || dependency
            .config
            .tags
            .iter()
            .any(|tag| required_tags.contains(tag))
    {
        return Ok(());
    }

    let mut allowed = Vec::from(required_tags);
    allowed.push(source_tag.to_owned());

    Err(EnforcerError::InvalidTagRelationship(
        source.id.to_string(),
        source_tag.clone(),
        dependency.id.to_string(),
        allowed.join(", "),
    ))
}
