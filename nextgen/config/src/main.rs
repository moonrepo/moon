use moon_config::{
    PartialInheritedTasksConfig, PartialProjectConfig, PartialTemplateConfig,
    PartialTemplateFrontmatterConfig, PartialToolchainConfig, PartialWorkspaceConfig,
};
use schemars::schema_for;
use std::fs;

fn main() {
    // Generate JSON schemas derived from our structs
    let project_schema = schema_for!(PartialProjectConfig);
    let tasks_schema = schema_for!(PartialInheritedTasksConfig);
    let template_schema = schema_for!(PartialTemplateConfig);
    let template_frontmatter_schema = schema_for!(PartialTemplateFrontmatterConfig);
    let toolchain_schema = schema_for!(PartialToolchainConfig);
    let workspace_schema = schema_for!(PartialWorkspaceConfig);

    fs::write(
        "website/static/schemas/project.json",
        serde_json::to_string_pretty(&project_schema).unwrap(),
    )
    .unwrap();

    fs::write(
        "website/static/schemas/tasks.json",
        serde_json::to_string_pretty(&tasks_schema).unwrap(),
    )
    .unwrap();

    fs::write(
        "website/static/schemas/template.json",
        serde_json::to_string_pretty(&template_schema).unwrap(),
    )
    .unwrap();

    fs::write(
        "website/static/schemas/template-frontmatter.json",
        serde_json::to_string_pretty(&template_frontmatter_schema).unwrap(),
    )
    .unwrap();

    fs::write(
        "website/static/schemas/toolchain.json",
        serde_json::to_string_pretty(&toolchain_schema).unwrap(),
    )
    .unwrap();

    fs::write(
        "website/static/schemas/workspace.json",
        serde_json::to_string_pretty(&workspace_schema).unwrap(),
    )
    .unwrap();
}
