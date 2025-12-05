use crate::app::Commands;
use crate::session::MoonSession;
use iocraft::prelude::element;
use miette::IntoDiagnostic;
use moon_action::Action;
use moon_action_context::ActionContext;
use moon_action_graph::ActionGraph;
use moon_action_pipeline::ActionPipeline;
use moon_common::Id;
use moon_console::ui::{OwnedOrShared, Progress, ProgressDisplay, ProgressReporter};
use moon_console::{Console, ConsoleError};
use moon_workspace::WorkspaceBuilderContext;
use serde::Serialize;
use starbase_utils::{fs, json, toml, yaml};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub fn serialize_config_based_on_extension(
    plugin_id: &Id,
    path: &Path,
    config: impl Serialize,
) -> miette::Result<String> {
    let template = match path.extension().and_then(|ext| ext.to_str()) {
        Some("json" | "jsonc") => json::format(
            &json::JsonMap::from_iter([(
                plugin_id.to_string(),
                json::serde_json::to_value(config).into_diagnostic()?,
            )]),
            true,
        )
        .into_diagnostic()?,
        Some("toml") => toml::format(
            &toml::TomlTable::from_iter([(
                plugin_id.to_string(),
                toml::TomlValue::try_from(config).into_diagnostic()?,
            )]),
            true,
        )
        .into_diagnostic()?,
        Some("yml" | "yaml") => yaml::format(&yaml::YamlMapping::from_iter([(
            yaml::YamlValue::String(plugin_id.to_string()),
            yaml::serde_yaml::to_value(config).into_diagnostic()?,
        )]))
        .into_diagnostic()?,
        _ => unimplemented!(),
    };

    Ok(template)
}

pub fn append_plugin_to_config_file(
    plugin_id: &Id,
    config_paths: Vec<PathBuf>,
    config: impl Serialize,
) -> miette::Result<PathBuf> {
    let path = config_paths
        .iter()
        .find(|path| path.exists())
        .unwrap_or(&config_paths[0]);

    fs::append_file(
        path,
        format!(
            "\n\n{}",
            serialize_config_based_on_extension(plugin_id, path, config)?
        ),
    )?;

    Ok(path.to_path_buf())
}

pub async fn run_action_pipeline(
    session: &MoonSession,
    action_context: ActionContext,
    action_graph: ActionGraph,
) -> miette::Result<Vec<Action>> {
    let mut pipeline = ActionPipeline::new(
        session.get_app_context().await?,
        session.get_workspace_graph().await?,
    );

    if let Some(concurrency) = &session.cli.concurrency {
        pipeline.concurrency = *concurrency;
    }

    match &session.cli.command {
        Commands::Check(cmd) => {
            pipeline.bail = true;
            pipeline.summarize = cmd.summary;
        }
        Commands::Ci(_) => {
            pipeline.report_name = "ciReport.json".into();
            pipeline.summarize = true;
        }
        Commands::Run(cmd) => {
            pipeline.bail = !cmd.no_bail;
            pipeline.summarize = cmd.summary;
        }
        Commands::Setup | Commands::Sync { .. } => {
            pipeline.summarize = true;
        }
        _ => {}
    };

    let results = pipeline
        .run_with_context(action_graph, action_context)
        .await?;

    Ok(results)
}

pub async fn create_workspace_graph_context(
    session: &MoonSession,
) -> miette::Result<WorkspaceBuilderContext<'_>> {
    let context = WorkspaceBuilderContext {
        config_loader: &session.config_loader,
        enabled_toolchains: session.toolchains_config.get_enabled(),
        extensions_config: &session.extensions_config,
        extension_registry: session.get_extension_registry().await?,
        inherited_tasks: &session.tasks_config,
        toolchains_config: &session.toolchains_config,
        toolchain_registry: session.get_toolchain_registry().await?,
        vcs: Some(session.get_vcs_adapter()?),
        working_dir: &session.working_dir,
        workspace_config: &session.workspace_config,
        workspace_root: &session.workspace_root,
    };

    Ok(context)
}

pub async fn create_progress_loader(
    console: Arc<Console>,
    message: impl AsRef<str>,
) -> ProgressInstance {
    let reporter = Arc::new(ProgressReporter::default());
    let reporter_clone = OwnedOrShared::Shared(reporter.clone());
    let message = message.as_ref().to_owned();

    let handle = tokio::task::spawn(async move {
        console
            .render_prompt(element! {
                Progress(
                    default_message: message,
                    display: ProgressDisplay::Loader,
                    reporter: reporter_clone,
                )
            })
            .await
    });

    // Wait a bit for the component to be rendered
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    ProgressInstance { handle, reporter }
}

pub struct ProgressInstance {
    pub handle: tokio::task::JoinHandle<Result<(), ConsoleError>>,
    pub reporter: Arc<ProgressReporter>,
}

impl ProgressInstance {
    pub async fn stop(self) -> miette::Result<()> {
        self.reporter.exit();
        self.handle.await.into_diagnostic()??;

        Ok(())
    }
}

impl Deref for ProgressInstance {
    type Target = ProgressReporter;

    fn deref(&self) -> &Self::Target {
        &self.reporter
    }
}
