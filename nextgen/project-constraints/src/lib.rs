use miette::Diagnostic;
use moon_common::{Id, Style, Stylize};
use moon_project::{Project, ProjectType};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum ProjectConstraintsError {
    #[error(
        "Invalid project relationship. Project {} of type {source_type} cannot depend on project {} of type {dep_type}; can only depend on {allowed}.",
        .source_id.style(Style::Id),
        .dep_id.style(Style::Id),
    )]
    InvalidTypeRelationship {
        source_id: Id,
        source_type: ProjectType,
        dep_id: Id,
        dep_type: ProjectType,
        allowed: String,
    },

    #[error(
        "Invalid tag relationship. Project {} with tag #{source_tag} cannot depend on project {}. The tag #{source_tag} requires a dependency to have one of the following tags: {allowed}.",
        .source_id.style(Style::Id),
        .dep_id.style(Style::Id),
    )]
    InvalidTagRelationship {
        source_id: Id,
        source_tag: Id,
        dep_id: Id,
        allowed: String,
    },
}

pub fn enforce_project_type_relationships(
    source: &Project,
    dependency: &Project,
) -> miette::Result<()> {
    let mut allowed = vec![];

    let valid = match source.type_of {
        ProjectType::Application => {
            allowed.push(ProjectType::Library.to_string());
            allowed.push(ProjectType::Tool.to_string());

            matches!(
                dependency.type_of,
                ProjectType::Library | ProjectType::Tool | ProjectType::Unknown
            )
        }
        ProjectType::Library | ProjectType::Tool => {
            allowed.push(ProjectType::Library.to_string());

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
        return Err(ProjectConstraintsError::InvalidTypeRelationship {
            source_id: source.id.clone(),
            source_type: source.type_of,
            dep_id: dependency.id.clone(),
            dep_type: dependency.type_of,
            allowed: allowed.join(", "),
        }
        .into());
    }

    Ok(())
}

pub fn enforce_tag_relationships(
    source: &Project,
    source_tag: &Id,
    dependency: &Project,
    required_tags: &[Id],
) -> miette::Result<()> {
    // Dependency project doesn't have any tags
    if required_tags.is_empty() {
        return Ok(());
    }

    // Source project isn't using the source tag
    if source_tag.is_empty()
        || source.config.tags.is_empty()
        || !source.config.tags.contains(source_tag)
    {
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

    Err(ProjectConstraintsError::InvalidTagRelationship {
        source_id: source.id.clone(),
        source_tag: source_tag.clone(),
        dep_id: dependency.id.clone(),
        allowed: allowed
            .iter()
            .map(|t| format!("#{t}"))
            .collect::<Vec<_>>()
            .join(", "),
    }
    .into())
}
