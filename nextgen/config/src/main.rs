use moon_config::*;
use schemars::schema_for;
use schematic::typescript::TypeScriptGenerator;
use std::fs;
use std::path::PathBuf;

fn generate_toolchain() {
    let toolchain_schema = schema_for!(PartialToolchainConfig);

    fs::write(
        "website/static/schemas/toolchain.json",
        serde_json::to_string_pretty(&toolchain_schema).unwrap(),
    )
    .unwrap();

    let mut generator =
        TypeScriptGenerator::new(PathBuf::from("packages/types/src/toolchain-config.ts"));

    generator.add_enum::<NodeProjectAliasFormat>();
    generator.add_enum::<NodeVersionFormat>();
    generator.add_enum::<NodePackageManager>();
    generator.add_enum::<NodeVersionManager>();
    generator.add::<DenoConfig>();
    generator.add::<NpmConfig>();
    generator.add::<PnpmConfig>();
    generator.add::<YarnConfig>();
    generator.add::<NodeConfig>();
    generator.add::<RustConfig>();
    generator.add::<TypeScriptConfig>();
    generator.add::<ToolchainConfig>();
    generator.add::<ToolchainConfig>();
    generator.add::<ToolchainConfig>();

    generator.generate().unwrap();
}

fn generate_workspace() {
    let workspace_schema = schema_for!(PartialWorkspaceConfig);

    fs::write(
        "website/static/schemas/workspace.json",
        serde_json::to_string_pretty(&workspace_schema).unwrap(),
    )
    .unwrap();

    let mut generator =
        TypeScriptGenerator::new(PathBuf::from("packages/types/src/workspace-config.ts"));

    generator.add_enum::<CodeownersOrderBy>();
    generator.add_enum::<HasherOptimization>();
    generator.add_enum::<HasherWalkStrategy>();
    generator.add_enum::<VcsManager>();
    generator.add_enum::<VcsProvider>();
    // generator.add_enum::<WorkspaceProjects>();
    generator.add::<CodeownersConfig>();
    generator.add::<ConstraintsConfig>();
    generator.add::<GeneratorConfig>();
    generator.add::<HasherConfig>();
    generator.add::<NotifierConfig>();
    generator.add::<RunnerConfig>();
    generator.add::<VcsConfig>();
    generator.add::<WorkspaceConfig>();
    generator.add::<WorkspaceConfig>();
    generator.add::<WorkspaceConfig>();
    generator.add::<WorkspaceConfig>();

    generator.generate().unwrap();
}

fn main() {
    // Generate JSON schemas derived from our structs
    let project_schema = schema_for!(PartialProjectConfig);
    let tasks_schema = schema_for!(PartialInheritedTasksConfig);
    let template_schema = schema_for!(PartialTemplateConfig);
    let template_frontmatter_schema = schema_for!(PartialTemplateFrontmatterConfig);

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

    generate_toolchain();
    generate_workspace();
}
