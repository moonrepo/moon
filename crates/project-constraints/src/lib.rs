use miette::Diagnostic;
use moon_common::{Id, Style, Stylize};
use moon_config::{DependencyScope, StackType};
use moon_project::{Project, LayerType};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum ProjectConstraintsError {
    #[diagnostic(code(project_constraints::invalid_type_relationship))]
    #[error(
        "Invalid project relationship. Project {} of type {source_type} cannot depend on project {} of type {dep_type}; can only depend on {allowed}.\n\nThis can be customized with the {} and {} settings.",
        .source_id.style(Style::Id),
        .dep_id.style(Style::Id),
        "stack".style(Style::Property),
        "type".style(Style::Property),
    )]
    InvalidTypeRelationship {
        source_id: Id,
        source_type: LayerType,
        dep_id: Id,
        dep_type: LayerType,
        allowed: String,
    },

    #[diagnostic(code(project_constraints::invalid_tag_relationship))]
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
    dependency_scope: &DependencyScope,
) -> miette::Result<()> {
    // These are special scopes that are implicitly applied by moon,
    // so don't take them into account when enforcing constraints.
    // Refer to project_builder for more information.
    if matches!(
        dependency_scope,
        DependencyScope::Build | DependencyScope::Root
    ) {
        return Ok(());
    }

    // We only want to enforce constraints when they are the same stack,
    // for example, frontend apps should not import from other frontend
    // apps, but frontend apps should depend on backend apps.
    if source.config.stack != dependency.config.stack
        && source.config.stack != StackType::Unknown
        && dependency.config.stack != StackType::Unknown
    {
        return Ok(());
    }

    let mut allowed = vec![
        LayerType::Configuration.to_string(),
        LayerType::Scaffolding.to_string(),
    ];

    let valid = match source.layer {
        LayerType::Application => {
            allowed.push(LayerType::Library.to_string());
            allowed.push(LayerType::Tool.to_string());

            matches!(
                dependency.layer,
                LayerType::Configuration
                    | LayerType::Scaffolding
                    | LayerType::Library
                    | LayerType::Tool
                    | LayerType::Unknown
            )
        }
        LayerType::Automation => {
            allowed.push(LayerType::Application.to_string());
            allowed.push(LayerType::Library.to_string());
            allowed.push(LayerType::Tool.to_string());

            !matches!(dependency.layer, LayerType::Automation)
        }
        LayerType::Library | LayerType::Tool => {
            allowed.push(LayerType::Library.to_string());

            matches!(
                dependency.layer,
                LayerType::Configuration
                    | LayerType::Scaffolding
                    | LayerType::Library
                    | LayerType::Unknown
            )
        }
        LayerType::Configuration | LayerType::Scaffolding => {
            matches!(
                dependency.layer,
                LayerType::Configuration | LayerType::Scaffolding
            )
        }
        LayerType::Unknown => {
            // Do nothing?
            true
        }
    };

    if !valid {
        return Err(ProjectConstraintsError::InvalidTypeRelationship {
            source_id: source.id.clone(),
            source_type: source.layer,
            dep_id: dependency.id.clone(),
            dep_type: dependency.layer,
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
