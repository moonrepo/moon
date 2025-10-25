use crate::session::MoonSession;
use clap::Args;
use iocraft::prelude::element;
use moon_common::Id;
use moon_console::ui::{Input, Notice, Select, SelectOption, StyledText, Variant};
use moon_docker::*;
use moon_pdk_api::DefineDockerMetadataInput;
use moon_project::Project;
use starbase::AppResult;
use starbase_utils::fs;
use tracing::{debug, instrument};

#[derive(Args, Clone, Debug)]
pub struct DockerFileArgs {
    #[arg(help = "ID of project to create a Dockerfile for")]
    id: Id,

    #[arg(help = "Destination path, relative from the project root")]
    dest: Option<String>,

    #[arg(long, help = "Use default options instead of prompting")]
    defaults: bool,

    #[arg(long = "buildTask", help = "ID of a task to build the project")]
    build_task: Option<Id>,

    #[arg(long, help = "Base Docker image to use")]
    image: Option<String>,

    #[arg(long, help = "Do not prune dependencies in the build stage")]
    no_prune: bool,

    #[arg(long, help = "Do not setup dependencies in the build stage")]
    no_setup: bool,

    #[arg(
        long,
        help = "Do not use the toolchain and instead use system binaries"
    )]
    no_toolchain: bool,

    #[arg(long = "startTask", help = "ID of a task to run the project")]
    start_task: Option<Id>,
}

#[instrument(skip_all)]
pub async fn file(session: MoonSession, args: DockerFileArgs) -> AppResult {
    let workspace_graph = session.get_workspace_graph().await?;
    let workspace_docker = &session.workspace_config.docker;
    let console = &session.console;

    // Ensure the project exists
    let project = workspace_graph.get_project(&args.id)?;
    let project_docker = &project.config.docker;

    let mut task_ids = project
        .task_targets
        .iter()
        .map(|task| &task.task_id)
        .collect::<Vec<_>>();
    task_ids.sort();

    // Build the options
    let mut options = GenerateDockerfileOptions {
        disable_toolchain: args.no_toolchain,
        project: args.id,
        prune: if args.no_prune {
            false
        } else {
            project_docker
                .file
                .run_prune
                .or(workspace_docker.file.run_prune)
                .unwrap_or(true)
        },
        setup: if args.no_setup {
            false
        } else {
            project_docker
                .file
                .run_setup
                .or(workspace_docker.file.run_setup)
                .unwrap_or(true)
        },
        ..GenerateDockerfileOptions::default()
    };

    debug!("Gathering Dockerfile options");

    let base_image = get_base_image(&session, &project).await?;
    let default_image = project_docker
        .file
        .image
        .clone()
        .or_else(|| workspace_docker.file.image.clone())
        .unwrap_or(base_image);

    if let Some(image) = args.image {
        options.image = image;
    } else if args.defaults {
        options.image = default_image;
    } else {
        console
            .render_interactive(element! {
                Input(
                    label: "Docker image?",
                    default_value: default_image,
                    on_value: &mut options.image,
                )
            })
            .await?;
    }

    debug!(image = &options.image, "Using Docker image");

    let build_task_setting = project_docker
        .file
        .build_task
        .as_ref()
        .or(workspace_docker.file.build_task.as_ref());

    let build_task_id = if let Some(id) = &args.build_task {
        Some(id)
    } else if args.defaults {
        build_task_setting
    } else {
        let default_index = build_task_setting
            .and_then(|id| task_ids.iter().position(|cursor_id| cursor_id == &id));
        let mut index = default_index.unwrap_or(0);

        console
            .render_interactive(element! {
                Select(
                    label: "Build task?",
                    options: {
                        let mut options = task_ids.iter().map(SelectOption::new).collect::<Vec<_>>();
                        options.push(SelectOption::new("(none)"));
                        options
                    },
                    default_index,
                    on_index: &mut index,
                )
            })
            .await?;

        if index == task_ids.len() {
            None
        } else {
            Some(task_ids[index])
        }
    };

    if let Some(task_id) = build_task_id {
        let target = workspace_graph
            .get_task_from_project(&project.id, task_id)?
            .target
            .to_owned();

        debug!(task = target.as_str(), "Using build task");

        options.build_task = Some(target);
    } else {
        debug!("Not using a build task");
    }

    let start_task_setting = project_docker
        .file
        .start_task
        .as_ref()
        .or(workspace_docker.file.start_task.as_ref());

    let start_task_id = if let Some(id) = &args.start_task {
        Some(id)
    } else if args.defaults {
        start_task_setting
    } else {
        let default_index = start_task_setting
            .and_then(|id| task_ids.iter().position(|cursor_id| cursor_id == &id));
        let mut index = default_index.unwrap_or(0);

        console
            .render_interactive(element! {
                Select(
                    label: "Start task?",
                    options: {
                        let mut options = task_ids.iter().map(SelectOption::new).collect::<Vec<_>>();
                        options.push(SelectOption::new("(none)"));
                        options
                    },
                    default_index,
                    on_index: &mut index,
                )
            })
            .await?;

        if index == task_ids.len() {
            None
        } else {
            Some(task_ids[index])
        }
    };

    if let Some(task_id) = start_task_id {
        let target = workspace_graph
            .get_task_from_project(&project.id, task_id)?
            .target
            .to_owned();

        debug!(task = target.as_str(), "Using start task");

        options.start_task = Some(target);
    } else {
        debug!("Not using a start task");
    }

    // Generate the file
    let out_file = project
        .root
        .join(args.dest.as_deref().unwrap_or("Dockerfile"));

    debug!(
        dockerfile = ?out_file,
        project_id = options.project.as_str(),
        "Generating Dockerfile in project",
    );

    fs::write_file(&out_file, generate_dockerfile(options)?)?;

    console.render(element! {
        Notice(variant: Variant::Success) {
            StyledText(
                content: format!("Generated <path>{}</path>", out_file.display())
            )
        }
    })?;

    Ok(None)
}

async fn get_base_image(session: &MoonSession, project: &Project) -> miette::Result<String> {
    let toolchain_registry = session.get_toolchain_registry().await?;

    for toolchain in toolchain_registry.load_many(&project.toolchains).await? {
        if toolchain.has_func("define_docker_metadata").await {
            let metadata = toolchain
                .define_docker_metadata(DefineDockerMetadataInput {
                    context: toolchain_registry.create_context(),
                    toolchain_config: toolchain_registry
                        .create_merged_config(&toolchain.id, &project.config),
                })
                .await?;

            if let Some(image) = metadata.default_image {
                return Ok(image);
            }
        }
    }

    Ok("scratch".into())
}
