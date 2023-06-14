#![allow(clippy::disallowed_types)]

use moon_config::*;
use schematic::renderers::json_schema::JsonSchemaRenderer;
use schematic::renderers::typescript::{TypeScriptOptions, TypeScriptRenderer};
use schematic::schema::SchemaGenerator;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

fn generate_project() {
    let mut generator = SchemaGenerator::default();

    generator.add::<PartialProjectConfig>();

    generator
        .generate(
            PathBuf::from("website/static/schemas/project.json"),
            JsonSchemaRenderer::default(),
        )
        .unwrap();

    generator.add::<DependencyConfig>();
    generator.add::<ProjectConfig>();

    generator
        .generate(
            PathBuf::from("packages/types/src/project-config.ts"),
            TypeScriptRenderer::new(TypeScriptOptions {
                exclude_references: HashSet::from_iter([
                    "PlatformType".into(),
                    "TaskCommandArgs".into(),
                    "TaskMergeStrategy".into(),
                    "TaskOutputStyle".into(),
                    "TaskType".into(),
                    "PartialTaskOptionsConfig".into(),
                    "PartialTaskConfig".into(),
                    "TaskOptionsConfig".into(),
                    "TaskConfig".into(),
                ]),
                external_types: HashMap::from_iter([(
                    "./tasks-config".into(),
                    HashSet::from_iter([
                        "PlatformType".into(),
                        "PartialTaskConfig".into(),
                        "TaskConfig".into(),
                    ]),
                )]),
                ..Default::default()
            }),
        )
        .unwrap();
}

fn generate_tasks() {
    let mut generator = SchemaGenerator::default();

    generator.add::<PartialInheritedTasksConfig>();

    generator
        .generate(
            PathBuf::from("website/static/schemas/tasks.json"),
            JsonSchemaRenderer::default(),
        )
        .unwrap();

    generator.add::<InheritedTasksConfig>();

    generator
        .generate(
            PathBuf::from("packages/types/src/tasks-config.ts"),
            TypeScriptRenderer::default(),
        )
        .unwrap();
}

fn generate_template() {
    let mut generator = SchemaGenerator::default();
    generator.add::<PartialTemplateConfig>();
    generator
        .generate(
            PathBuf::from("website/static/schemas/template.json"),
            JsonSchemaRenderer::default(),
        )
        .unwrap();

    let mut generator = SchemaGenerator::default();
    generator.add::<PartialTemplateFrontmatterConfig>();
    generator
        .generate(
            PathBuf::from("website/static/schemas/template-frontmatter.json"),
            JsonSchemaRenderer::default(),
        )
        .unwrap();
}

fn generate_toolchain() {
    let mut generator = SchemaGenerator::default();

    generator.add::<PartialToolchainConfig>();

    generator
        .generate(
            PathBuf::from("website/static/schemas/toolchain.json"),
            JsonSchemaRenderer::default(),
        )
        .unwrap();

    generator.add::<ToolchainConfig>();

    generator
        .generate(
            PathBuf::from("packages/types/src/toolchain-config.ts"),
            TypeScriptRenderer::default(),
        )
        .unwrap();
}

fn generate_workspace() {
    let mut generator = SchemaGenerator::default();

    generator.add::<PartialWorkspaceConfig>();

    generator
        .generate(
            PathBuf::from("website/static/schemas/workspace.json"),
            JsonSchemaRenderer::default(),
        )
        .unwrap();

    generator.add::<WorkspaceConfig>();

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
}
