use moon_config::{GlobalProjectConfig, ProjectConfig, WorkspaceConfig};
use schemars::schema_for;
use std::fs;

fn main() {
    // Generate JSON schemas derived from our structs
    let project_schema = schema_for!(ProjectConfig);
    let global_project_schema = schema_for!(GlobalProjectConfig);
    let workspace_schema = schema_for!(WorkspaceConfig);

    fs::write(
        "schemas/project.json",
        serde_json::to_string_pretty(&project_schema).unwrap(),
    )
    .unwrap();

    fs::write(
        "schemas/global-project.json",
        serde_json::to_string_pretty(&global_project_schema).unwrap(),
    )
    .unwrap();

    fs::write(
        "schemas/workspace.json",
        serde_json::to_string_pretty(&workspace_schema).unwrap(),
    )
    .unwrap();
}
