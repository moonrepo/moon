use moon_common::Id;
use moon_config::{
    ExtensionPluginConfig, ExtensionsConfig, ProjectConfig, ProjectDependencyConfig,
    TaskDependencyConfig, ToolchainPluginConfig, ToolchainsConfig, WorkspaceConfig,
};
use moon_graph_utils::GraphExpanderContext;
use moon_project::{FileGroup, Project};
use moon_task::{Task, TaskCheck, TaskOptions};
use moon_time::{now_millis, now_timestamp};
use rustc_hash::FxHashMap;
use serde::Serialize;
use std::collections::BTreeMap;
use std::env::consts;
use std::path::PathBuf;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostContext {
    pub os: &'static str,
    pub arch: &'static str,
    pub family: &'static str,
}

impl HostContext {
    pub fn new() -> Self {
        Self {
            os: consts::OS,
            arch: consts::ARCH,
            family: consts::FAMILY,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceContext<'ws> {
    pub config: &'ws WorkspaceConfig,
    pub config_dir: &'ws PathBuf,
    pub root: &'ws PathBuf,
    pub working_dir: &'ws PathBuf,
}

impl<'ws> WorkspaceContext<'ws> {
    pub fn new(graph: &'ws GraphExpanderContext) -> Self {
        Self {
            config: &graph.workspace_config,
            config_dir: &graph.config_dir,
            root: &graph.workspace_root,
            working_dir: &graph.working_dir,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionsContext<'ext> {
    pub config: &'ext ExtensionsConfig,
    pub plugins: FxHashMap<&'ext Id, &'ext ExtensionPluginConfig>,
}

impl<'ext> ExtensionsContext<'ext> {
    pub fn new(context: &'ext GraphExpanderContext) -> Self {
        Self {
            plugins: context.extensions_config.plugins.iter().collect(),
            config: &context.extensions_config,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolchainsContext<'tc> {
    pub config: &'tc ToolchainsConfig,
    pub plugins: FxHashMap<&'tc Id, &'tc ToolchainPluginConfig>,
}

impl<'tc> ToolchainsContext<'tc> {
    pub fn new(context: &'tc GraphExpanderContext) -> Self {
        Self {
            plugins: context.toolchains_config.plugins.iter().collect(),
            config: &context.toolchains_config,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VcsContext<'vcs> {
    pub branch: &'vcs String,
    pub revision: &'vcs String,
    pub repository: &'vcs String,
}

impl<'vcs> VcsContext<'vcs> {
    pub fn new(graph: &'vcs GraphExpanderContext) -> Self {
        Self {
            branch: &graph.vcs_branch,
            revision: &graph.vcs_revision,
            repository: &graph.vcs_repository,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatetimeContext {
    pub date: String,
    pub time: String,
    pub datetime: String,
    pub timestamp: u128,
}

impl DatetimeContext {
    pub fn new() -> Self {
        let now = now_timestamp();

        Self {
            date: now.format("%F").to_string(),
            datetime: now.format("%F_%T").to_string(),
            time: now.format("%T").to_string(),
            timestamp: (now_millis() / 1000),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectContext<'proj> {
    pub aliases: Vec<&'proj String>,
    pub config: &'proj ProjectConfig,
    pub dependencies: Vec<&'proj ProjectDependencyConfig>,
    pub env: BTreeMap<&'proj String, &'proj String>,
    pub file_groups: BTreeMap<&'proj Id, &'proj FileGroup>,
    pub id: &'proj Id,
    pub language: String,
    pub layer: String,
    pub root: &'proj PathBuf,
    pub source: &'proj str,
    pub stack: String,
    pub tags: Vec<&'proj Id>,
    pub task_targets: Vec<&'proj str>,
    pub toolchains: Vec<&'proj Id>,

    // Metadata
    pub title: Option<&'proj String>,
    pub description: Option<&'proj String>,
    pub owner: Option<&'proj String>,
    pub maintainers: Vec<&'proj String>,
    pub channel: Option<&'proj String>,
    pub metadata: FxHashMap<&'proj String, &'proj serde_json::Value>,
}

impl<'proj> ProjectContext<'proj> {
    pub fn new(project: &'proj Project) -> Self {
        let metadata = project.config.project.as_ref();

        Self {
            aliases: project.aliases.iter().map(|alias| &alias.alias).collect(),
            config: &project.config,
            dependencies: project.dependencies.iter().collect(),
            env: project
                .config
                .env
                .iter()
                .filter_map(|(k, v)| v.as_ref().map(|v| (k, v)))
                .collect(),
            file_groups: project.file_groups.iter().collect(),
            id: &project.id,
            language: project.language.to_string(),
            layer: project.layer.to_string(),
            root: &project.root,
            source: project.source.as_str(),
            stack: project.stack.to_string(),
            tags: project.config.tags.iter().collect(),
            task_targets: project
                .task_targets
                .iter()
                .map(|target| target.id.as_str())
                .collect(),
            toolchains: project.toolchains.iter().collect(),

            // Metadata
            title: metadata.and_then(|meta| meta.title.as_ref()),
            description: metadata.and_then(|meta| meta.description.as_ref()),
            owner: metadata.and_then(|meta| meta.owner.as_ref()),
            maintainers: metadata
                .map(|meta| meta.maintainers.iter().collect())
                .unwrap_or_default(),
            channel: metadata.and_then(|meta| meta.channel.as_ref()),
            metadata: metadata
                .map(|meta| meta.metadata.iter().collect())
                .unwrap_or_default(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskContext<'task> {
    pub command: &'task String,
    pub script: Option<&'task String>,
    pub args: Vec<&'task String>,
    pub checks: Vec<&'task TaskCheck>,
    pub deps: Vec<&'task TaskDependencyConfig>,
    pub env: BTreeMap<&'task String, &'task String>,
    pub id: &'task Id,
    pub input_files: Vec<&'task str>,
    pub input_globs: Vec<&'task str>,
    pub output_files: Vec<&'task str>,
    pub output_globs: Vec<&'task str>,
    pub options: &'task TaskOptions,
    pub preset: Option<String>,
    pub tags: Vec<&'task Id>,
    pub target: &'task str,
    pub toolchains: Vec<&'task Id>,
    #[serde(rename = "type")]
    pub type_of: String,
}

impl<'task> TaskContext<'task> {
    pub fn new(task: &'task Task) -> Self {
        Self {
            command: &task.command,
            script: task.script.as_ref(),
            args: task
                .args
                .iter()
                .map(|arg| arg.quoted_value.as_ref().unwrap_or(&arg.value))
                .collect(),
            checks: task.checks.iter().collect(),
            deps: task.deps.iter().collect(),
            env: task
                .env
                .iter()
                .filter_map(|(k, v)| v.as_ref().map(|v| (k, v)))
                .collect(),
            id: &task.id,
            input_files: task
                .input_files
                .keys()
                .map(|input| input.as_str())
                .collect(),
            input_globs: task.input_globs.keys().map(|glob| glob.as_str()).collect(),
            output_files: task
                .output_files
                .keys()
                .map(|output| output.as_str())
                .collect(),
            output_globs: task.output_globs.keys().map(|glob| glob.as_str()).collect(),
            options: &task.options,
            preset: task.preset.as_ref().map(|preset| preset.to_string()),
            tags: task.tags.iter().collect(),
            target: task.target.as_str(),
            toolchains: task.toolchains.iter().collect(),
            type_of: task.type_of.to_string(),
        }
    }
}
