#![allow(clippy::disallowed_types)]

use moon_config::*;
use schematic::schema::json_schema::JsonSchemaRenderer;
use schematic::schema::typescript::{TypeScriptOptions, TypeScriptRenderer};
use schematic::schema::{JsonSchemaOptions, SchemaGenerator};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

fn create_jsonschema_renderer() -> JsonSchemaRenderer<'static> {
    JsonSchemaRenderer::new(JsonSchemaOptions {
        markdown_description: true,
        mark_struct_fields_required: false,
        set_field_name_as_title: true,
        ..JsonSchemaOptions::default()
    })
}

fn generate_project() {
    let mut generator = SchemaGenerator::default();

    generator.add::<ProjectConfig>();

    generator
        .generate(
            PathBuf::from("website/static/schemas/project.json"),
            create_jsonschema_renderer(),
        )
        .unwrap();

    generator.add::<DependencyConfig>();
    generator.add::<PartialProjectConfig>();

    generator
        .generate(
            PathBuf::from("packages/types/src/project-config.ts"),
            TypeScriptRenderer::new(TypeScriptOptions {
                exclude_references: vec![
                    "PartialTaskArgs".into(),
                    "PartialTaskConfig".into(),
                    "PartialTaskDependency".into(),
                    "PartialTaskDependencyConfig".into(),
                    "PartialTaskOptionsConfig".into(),
                    "PlatformType".into(),
                    "TaskArgs".into(),
                    "TaskConfig".into(),
                    "TaskDependency".into(),
                    "TaskDependencyConfig".into(),
                    "TaskMergeStrategy".into(),
                    "TaskOptionAffectedFiles".into(),
                    "TaskOptionEnvFile".into(),
                    "TaskOptionsConfig".into(),
                    "TaskOutputStyle".into(),
                    "TaskUnixShell".into(),
                    "TaskWindowsShell".into(),
                    "TaskType".into(),
                ],
                external_types: HashMap::from_iter([(
                    "./tasks-config".into(),
                    vec![
                        "PlatformType".into(),
                        "PartialTaskConfig".into(),
                        "TaskConfig".into(),
                    ],
                )]),
                ..Default::default()
            }),
        )
        .unwrap();
}

fn generate_tasks() {
    let mut generator = SchemaGenerator::default();

    generator.add::<InheritedTasksConfig>();

    generator
        .generate(
            PathBuf::from("website/static/schemas/tasks.json"),
            create_jsonschema_renderer(),
        )
        .unwrap();

    generator.add::<PartialInheritedTasksConfig>();

    generator
        .generate(
            PathBuf::from("packages/types/src/tasks-config.ts"),
            TypeScriptRenderer::default(),
        )
        .unwrap();
}

fn generate_template() {
    let mut generator = SchemaGenerator::default();
    generator.add::<TemplateConfig>();
    generator
        .generate(
            PathBuf::from("website/static/schemas/template.json"),
            create_jsonschema_renderer(),
        )
        .unwrap();

    let mut generator = SchemaGenerator::default();
    generator.add::<TemplateFrontmatterConfig>();
    generator
        .generate(
            PathBuf::from("website/static/schemas/template-frontmatter.json"),
            create_jsonschema_renderer(),
        )
        .unwrap();

    let mut generator = SchemaGenerator::default();
    generator.add::<PartialTemplateConfig>();
    generator.add::<PartialTemplateFrontmatterConfig>();
    generator.add::<TemplateConfig>();
    generator.add::<TemplateFrontmatterConfig>();
    generator
        .generate(
            PathBuf::from("packages/types/src/template-config.ts"),
            TypeScriptRenderer::default(),
        )
        .unwrap();
}

fn generate_toolchain() {
    let mut generator = SchemaGenerator::default();

    generator.add::<ToolchainConfig>();

    generator
        .generate(
            PathBuf::from("website/static/schemas/toolchain.json"),
            create_jsonschema_renderer(),
        )
        .unwrap();

    generator.add::<PartialToolchainConfig>();

    generator
        .generate(
            PathBuf::from("packages/types/src/toolchain-config.ts"),
            TypeScriptRenderer::default(),
        )
        .unwrap();
}

fn generate_workspace() {
    let mut generator = SchemaGenerator::default();

    generator.add::<WorkspaceConfig>();

    generator
        .generate(
            PathBuf::from("website/static/schemas/workspace.json"),
            create_jsonschema_renderer(),
        )
        .unwrap();

    generator.add::<PartialWorkspaceConfig>();

    generator
        .generate(
            PathBuf::from("packages/types/src/workspace-config.ts"),
            TypeScriptRenderer::default(),
        )
        .unwrap();
}

fn main() {
    generate_project();
    generate_tasks();
    generate_template();
    generate_toolchain();
    generate_workspace();

    // Run prettier
    let prettier = PathBuf::from("node_modules/.bin/prettier");

    if prettier.exists() {
        let mut cmd = Command::new(prettier);
        cmd.args(["--write", "packages/types"]);
        cmd.output().unwrap();
    }
}
