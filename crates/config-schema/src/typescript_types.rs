#![allow(clippy::disallowed_types)]

use moon_config::*;
use schematic::schema::SchemaGenerator;
use schematic::schema::typescript::{TypeScriptOptions, TypeScriptRenderer};
use std::collections::HashMap;
use std::path::Path;

fn generate_extensions(out_dir: &Path) -> miette::Result<()> {
    let mut generator = SchemaGenerator::default();
    generator.add::<ExtensionsConfig>();
    generator.generate(
        out_dir.join("extensions-config.ts"),
        TypeScriptRenderer::new(TypeScriptOptions {
            exclude_references: vec!["Id".into(), "ExtendsFrom".into(), "PluginLocator".into()],
            external_types: HashMap::from_iter([
                ("./common".into(), vec!["Id".into(), "ExtendsFrom".into()]),
                ("./toolchains-config".into(), vec!["PluginLocator".into()]),
            ]),
            ..Default::default()
        }),
    )
}

fn generate_project(out_dir: &Path) -> miette::Result<()> {
    let mut generator = SchemaGenerator::default();
    generator.add::<ProjectDependencyConfig>();
    generator.add::<ProjectConfig>();
    generator.add::<PartialProjectConfig>();
    generator.generate(
        out_dir.join("project-config.ts"),
        TypeScriptRenderer::new(TypeScriptOptions {
            exclude_references: vec![
                "DockerFileConfig".into(),
                "DockerScaffoldConfig".into(),
                "Id".into(),
                "Input".into(),
                "FileInput".into(),
                "FileGroupInput".into(),
                "FileGroupInputFormat".into(),
                "FileOutput".into(),
                "Output".into(),
                "GlobInput".into(),
                "GlobOutput".into(),
                "ProjectInput".into(),
                "PartialDockerFileConfig".into(),
                "PartialDockerScaffoldConfig".into(),
                "PartialTaskArgs".into(),
                "PartialTaskConfig".into(),
                "PartialTaskDependency".into(),
                "PartialTaskDependencyConfig".into(),
                "PartialTaskOptionsConfig".into(),
                "PartialTaskOptionAffectedFilesConfig".into(),
                "PartialToolchainPluginConfig".into(),
                "PluginLocator".into(),
                "TaskArgs".into(),
                "TaskConfig".into(),
                "TaskDependency".into(),
                "TaskDependencyConfig".into(),
                "TaskMergeStrategy".into(),
                "TaskOperatingSystem".into(),
                "TaskOptionAffectedFiles".into(),
                "TaskOptionAffectedFilesConfig".into(),
                "TaskOptionAffectedFilesEntry".into(),
                "TaskOptionEnvFile".into(),
                "TaskOptionsConfig".into(),
                "TaskOutputStyle".into(),
                "TaskPreset".into(),
                "TaskPriority".into(),
                "TaskUnixShell".into(),
                "TaskWindowsShell".into(),
                "TaskType".into(),
                "ToolchainPluginConfig".into(),
                "ToolchainPluginVersionFrom".into(),
                "UnresolvedVersionSpec".into(),
            ],
            external_types: HashMap::from_iter([
                (
                    "./tasks-config".into(),
                    vec![
                        "Input".into(),
                        "PartialTaskConfig".into(),
                        "TaskConfig".into(),
                    ],
                ),
                (
                    "./toolchains-config".into(),
                    vec![
                        "PartialToolchainPluginConfig".into(),
                        "ToolchainPluginConfig".into(),
                    ],
                ),
                (
                    "./workspace-config".into(),
                    vec![
                        "DockerFileConfig".into(),
                        "DockerScaffoldConfig".into(),
                        "PartialDockerFileConfig".into(),
                        "PartialDockerScaffoldConfig".into(),
                    ],
                ),
                ("./common".into(), vec!["Id".into()]),
            ]),
            ..Default::default()
        }),
    )
}

fn generate_tasks(out_dir: &Path) -> miette::Result<()> {
    let mut generator = SchemaGenerator::default();
    generator.add::<TaskDependencyType>();
    generator.add::<InheritedTasksConfig>();
    generator.add::<PartialInheritedTasksConfig>();
    generator.generate(
        out_dir.join("tasks-config.ts"),
        TypeScriptRenderer::new(TypeScriptOptions {
            exclude_references: vec![
                "Id".into(),
                "ExtendsFrom".into(),
                "LanguageType".into(),
                "LayerType".into(),
                "StackType".into(),
            ],
            external_types: HashMap::from_iter([
                ("./common".into(), vec!["Id".into(), "ExtendsFrom".into()]),
                (
                    "./project-config".into(),
                    vec![
                        "LanguageType".into(),
                        "LayerType".into(),
                        "StackType".into(),
                    ],
                ),
            ]),
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
        TypeScriptRenderer::new(TypeScriptOptions {
            exclude_references: vec!["Id".into()],
            external_types: HashMap::from_iter([("./common".into(), vec!["Id".into()])]),
            ..Default::default()
        }),
    )
}

fn generate_toolchains(out_dir: &Path) -> miette::Result<()> {
    let mut generator = SchemaGenerator::default();
    generator.add::<ToolchainsConfig>();
    generator.add::<PartialToolchainsConfig>();
    generator.generate(
        out_dir.join("toolchains-config.ts"),
        TypeScriptRenderer::new(TypeScriptOptions {
            exclude_references: vec!["Id".into(), "ExtendsFrom".into()],
            external_types: HashMap::from_iter([(
                "./common".into(),
                vec!["Id".into(), "ExtendsFrom".into()],
            )]),
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
            exclude_references: vec!["Id".into(), "ExtendsFrom".into()],
            external_types: HashMap::from_iter([(
                "./common".into(),
                vec!["Id".into(), "ExtendsFrom".into()],
            )]),
            ..Default::default()
        }),
    )
}

pub fn generate_typescript_types(out_dir: impl AsRef<Path>) -> miette::Result<()> {
    let out_dir = out_dir.as_ref();

    generate_extensions(out_dir)?;
    generate_project(out_dir)?;
    generate_tasks(out_dir)?;
    generate_template(out_dir)?;
    generate_toolchains(out_dir)?;
    generate_workspace(out_dir)?;

    Ok(())
}
