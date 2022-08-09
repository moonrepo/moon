mod hasher;
pub mod task;

pub use hasher::NodeTargetHasher;
use moon_config::{NodeProjectAliasFormat, ProjectAlias, ProjectID};
use moon_lang_node::node::parse_package_name;
use moon_lang_node::package::PackageJson;
use moon_logger::{color, warn};
use moon_task::TaskError;
use std::collections::HashMap;
use std::path::Path;
use task::{ScriptParser, TasksMap};

pub const LOG_TARGET: &str = "moon:platform-node";

pub fn create_tasks_from_scripts(
    project_id: &str,
    package_json: &mut PackageJson,
) -> Result<TasksMap, TaskError> {
    let mut parser = ScriptParser::new(project_id);

    parser.parse_scripts(package_json)?;
    parser.update_package(package_json)?;

    Ok(parser.tasks)
}

pub fn infer_tasks_from_scripts(
    project_id: &str,
    package_json: &PackageJson,
) -> Result<TasksMap, TaskError> {
    let mut parser = ScriptParser::new(project_id);

    parser.infer_scripts(package_json)?;

    Ok(parser.tasks)
}

pub fn infer_tasks_from_package(
    project_id: &str,
    project_root: &Path,
) -> Result<Option<TasksMap>, TaskError> {
    if let Some(package_json) = PackageJson::read(project_root)? {
        return Ok(Some(infer_tasks_from_scripts(project_id, &package_json)?));
    }

    Ok(None)
}

pub fn load_project_aliases_from_packages(
    workspace_root: &Path,
    projects_map: &HashMap<ProjectID, String>,
    alias_format: &NodeProjectAliasFormat,
    aliases_map: &mut HashMap<ProjectAlias, ProjectID>,
) -> Result<(), TaskError> {
    for (project_id, project_source) in projects_map {
        if let Some(package_json) = PackageJson::read(workspace_root.join(project_source))? {
            if let Some(package_name) = package_json.name {
                let alias = match alias_format {
                    NodeProjectAliasFormat::NameAndScope => package_name,
                    NodeProjectAliasFormat::NameOnly => parse_package_name(&package_name).1,
                };

                if let Some(existing_source) = projects_map.get(&alias) {
                    warn!(
                        target: LOG_TARGET,
                        "A project already exists with the ID {} ({}), skipping alias of the same name ({})",
                        color::id(alias),
                        color::file(existing_source),
                        color::file(project_source)
                    );

                    continue;
                }

                if let Some(existing_id) = aliases_map.get(&alias) {
                    warn!(
                        target: LOG_TARGET,
                        "A project already exists with the alias {} (for project {}), skipping conflicting alias",
                        color::id(alias),
                        color::id(existing_id)
                    );

                    continue;
                }

                aliases_map.insert(alias, project_id.to_owned());
            }
        }
    }

    Ok(())
}
