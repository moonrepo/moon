#![allow(clippy::disallowed_types)]

use moon_config::*;
use schematic::schema::SchemaGenerator;
use schematic::schema::typescript::{TypeScriptOptions, TypeScriptRenderer};
use std::collections::HashMap;
use std::path::Path;

fn generate_project(out_dir: &Path) -> miette::Result<()> {
    let mut generator = SchemaGenerator::default();
    generator.add::<DependencyType>();
    generator.add::<DependencyConfig>();
    generator.add::<ProjectConfig>();
    generator.add::<PartialProjectConfig>();
    generator.generate(
        out_dir.join("project-config.ts"),
        TypeScriptRenderer::new(TypeScriptOptions {
            exclude_references: vec![
                "PartialTaskArgs".into(),
                "PartialTaskConfig".into(),
                "PartialTaskDependency".into(),
                "PartialTaskDependencyConfig".into(),
                "PartialTaskOptionsConfig".into(),
                "PartialToolchainPluginConfig".into(),
                "PlatformType".into(),
                "PluginLocator".into(),
                "TaskArgs".into(),
                "TaskConfig".into(),
                "TaskDependency".into(),
                "TaskDependencyConfig".into(),
                "TaskMergeStrategy".into(),
                "TaskOperatingSystem".into(),
                "TaskOptionAffectedFiles".into(),
                "TaskOptionEnvFile".into(),
                "TaskOptionsConfig".into(),
                "TaskOutputStyle".into(),
                "TaskPreset".into(),
                "TaskPriority".into(),
                "TaskUnixShell".into(),
                "TaskWindowsShell".into(),
                "TaskType".into(),
                "ToolchainPluginConfig".into(),
                "UnresolvedVersionSpec".into(),
            ],
            external_types: HashMap::from_iter([
                (
                    "./tasks-config".into(),
                    vec![
                        "PlatformType".into(),
                        "PartialTaskConfig".into(),
                        "TaskConfig".into(),
                    ],
                ),
                (
                    "./toolchain-config".into(),
                    vec![
                        "PartialToolchainPluginConfig".into(),
                        "ToolchainPluginConfig".into(),
                        "UnresolvedVersionSpec".into(),
                    ],
                ),
            ]),
            ..Default::default()
        }),
    )
}

fn generate_tasks(out_dir: &Path) -> miette::Result<()> {
    let mut generator = SchemaGenerator::default();
    generator.add::<InheritedTasksConfig>();
    generator.add::<PartialInheritedTasksConfig>();
    generator.generate(
        out_dir.join("tasks-config.ts"),
        TypeScriptRenderer::new(TypeScriptOptions {
            exclude_references: vec!["ExtendsFrom".into()],
            external_types: HashMap::from_iter([("./common".into(), vec!["ExtendsFrom".into()])]),
            ..Default::default()
        }),
    )
}

fn generate_template(out_dir: &Path) -> miette::Result<()> {
    let mut generator = SchemaGenerator::default();
    generator.add::<TemplateFrontmatterConfig>();
    generator.add::<PartialTemplateFrontmatterConfig>();
    generator.add::<TemplateConfig>();
    generator.add::<PartialTemplateConfig>();
    generator.generate(
        out_dir.join("template-config.ts"),
        TypeScriptRenderer::default(),
    )
}

fn generate_toolchain(out_dir: &Path) -> miette::Result<()> {
    let mut generator = SchemaGenerator::default();
    generator.add::<ToolchainConfig>();
    generator.add::<PartialToolchainConfig>();
    generator.generate(
        out_dir.join("toolchain-config.ts"),
        TypeScriptRenderer::new(TypeScriptOptions {
            exclude_references: vec!["ExtendsFrom".into()],
            external_types: HashMap::from_iter([("./common".into(), vec!["ExtendsFrom".into()])]),
            ..Default::default()
        }),
    )
}

fn generate_workspace(out_dir: &Path) -> miette::Result<()> {
    let mut generator = SchemaGenerator::default();
    generator.add::<WorkspaceConfig>();
    generator.add::<PartialWorkspaceConfig>();
    generator.generate(
        out_dir.join("workspace-config.ts"),
        TypeScriptRenderer::new(TypeScriptOptions {
            exclude_references: vec!["ExtendsFrom".into(), "PluginLocator".into()],
            external_types: HashMap::from_iter([
                ("./common".into(), vec!["ExtendsFrom".into()]),
                ("./toolchain-config".into(), vec!["PluginLocator".into()]),
            ]),
            ..Default::default()
        }),
    )
}

pub fn generate_typescript_types(out_dir: impl AsRef<Path>) -> miette::Result<()> {
    let out_dir = out_dir.as_ref();

    generate_project(out_dir)?;
    generate_tasks(out_dir)?;
    generate_template(out_dir)?;
    generate_toolchain(out_dir)?;
    generate_workspace(out_dir)?;

    Ok(())
}
