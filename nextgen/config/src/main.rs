use moon_config::*;
use schemars::schema_for;
use schematic::typescript::{Output, Type, TypeScriptGenerator};
use std::fs;
use std::path::PathBuf;

fn create_type_alias(name: &str) -> Output {
    Output::Enum {
        name,
        fields: vec![Output::Field {
            name: "".into(),
            value: Type::String,
            optional: false,
        }],
    }
}

fn generate_common() {
    let mut generator =
        TypeScriptGenerator::new(PathBuf::from("packages/types/src/common-config.ts"));

    generator.add_custom(create_type_alias("Id"));
    generator.add_custom(create_type_alias("Target"));
    generator.add_custom(create_type_alias("FilePath"));
    generator.add_custom(create_type_alias("GlobPath"));
    generator.add_custom(create_type_alias("InputPath"));
    generator.add_custom(create_type_alias("OutputPath"));
    generator.add_enum::<LanguageType>();
    generator.add_enum::<PlatformType>();

    generator.generate().unwrap();
}

fn generate_project() {
    let project_schema = schema_for!(PartialProjectConfig);

    fs::write(
        "website/static/schemas/project.json",
        serde_json::to_string_pretty(&project_schema).unwrap(),
    )
    .unwrap();

    let mut generator =
        TypeScriptGenerator::new(PathBuf::from("packages/types/src/project-config.ts"));

    generator.add_enum::<DependencyScope>();
    generator.add_enum::<DependencySource>();
    generator.add_enum::<ProjectType>();
    generator.add::<DependencyConfig>();
    generator.add::<ProjectMetadataConfig>();
    generator.add::<OwnersConfig>();
    generator.add::<ProjectToolchainCommonToolConfig>();
    generator.add::<ProjectToolchainTypeScriptConfig>();
    generator.add::<ProjectToolchainConfig>();
    generator.add::<ProjectWorkspaceInheritedTasksConfig>();
    generator.add::<ProjectWorkspaceConfig>();
    generator.add::<ProjectConfig>();

    generator.generate().unwrap();
}

fn generate_tasks() {
    let tasks_schema = schema_for!(PartialInheritedTasksConfig);

    fs::write(
        "website/static/schemas/tasks.json",
        serde_json::to_string_pretty(&tasks_schema).unwrap(),
    )
    .unwrap();

    let mut generator =
        TypeScriptGenerator::new(PathBuf::from("packages/types/src/tasks-config.ts"));

    generator.add_enum::<TaskMergeStrategy>();
    generator.add_enum::<TaskOutputStyle>();
    generator.add_enum::<TaskType>();
    generator.add::<InheritedTasksConfig>();
    generator.add::<InheritedTasksConfig>();
    generator.add::<TaskOptionsConfig>();
    generator.add::<TaskConfig>();
    generator.add::<InheritedTasksConfig>();

    generator.generate().unwrap();
}

fn generate_template() {
    let template_schema = schema_for!(PartialTemplateConfig);
    let template_frontmatter_schema = schema_for!(PartialTemplateFrontmatterConfig);

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
}

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
    generator.add::<CodeownersConfig>();
    generator.add::<ConstraintsConfig>();
    generator.add::<GeneratorConfig>();
    generator.add::<HasherConfig>();
    generator.add::<NotifierConfig>();
    generator.add::<RunnerConfig>();
    generator.add::<VcsConfig>();
    generator.add::<WorkspaceConfig>();

    generator.generate().unwrap();
}

fn main() {
    generate_common();
    generate_project();
    generate_tasks();
    generate_template();
    generate_toolchain();
    generate_workspace();
}
