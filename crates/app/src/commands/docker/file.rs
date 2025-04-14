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

    #[arg(long, help = "Do not prune the workspace in the build stage")]
    no_prune: bool,

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
    let console = &session.console;
    let workspace_graph = session.get_workspace_graph().await?;

    // Ensure the project exists
    let project = workspace_graph.get_project(&args.id)?;
    let tasks = workspace_graph.get_tasks_from_project(&project.id)?;

    // Build the options
    let mut options = GenerateDockerfileOptions {
        disable_toolchain: args.no_toolchain,
        project: args.id,
        prune: !args.no_prune,
        ..GenerateDockerfileOptions::default()
    };

    debug!("Gathering Dockerfile options");

    let base_image = get_base_image(&session, &project).await?;
    let default_image = project
        .config
        .docker
        .file
        .image
        .clone()
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

    let build_task_id = if let Some(id) = &args.build_task {
        Some(id)
    } else if args.defaults {
        project.config.docker.file.build_task.as_ref()
    } else {
        let mut ids = tasks.iter().map(|task| &task.id).collect::<Vec<_>>();
        ids.sort();

        let default_index = project
            .config
            .docker
            .file
            .build_task
            .as_ref()
            .and_then(|id| ids.iter().position(|cursor_id| cursor_id == &id));
        let mut index = default_index.unwrap_or(0);

        console
            .render_interactive(element! {
                Select(
                    label: "Build task?",
                    options: {
                        let mut options = ids.iter().map(SelectOption::new).collect::<Vec<_>>();
                        options.push(SelectOption::new("(none)"));
                        options
                    },
                    default_index,
                    on_index: &mut index,
                )
            })
            .await?;

        if index == ids.len() {
            None
        } else {
            Some(ids[index])
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

    let start_task_id = if let Some(id) = &args.start_task {
        Some(id)
    } else if args.defaults {
        project.config.docker.file.start_task.as_ref()
    } else {
        let mut ids = tasks.iter().map(|task| &task.id).collect::<Vec<_>>();
        ids.sort();

        let default_index = project
            .config
            .docker
            .file
            .start_task
            .as_ref()
            .and_then(|id| ids.iter().position(|cursor_id| cursor_id == &id));
        let mut index = default_index.unwrap_or(0);

        console
            .render_interactive(element! {
                Select(
                    label: "Start task?",
                    options: {
                        let mut options = ids.iter().map(SelectOption::new).collect::<Vec<_>>();
                        options.push(SelectOption::new("(none)"));
                        options
                    },
                    default_index,
                    on_index: &mut index,
                )
            })
            .await?;

        if index == ids.len() {
            None
        } else {
            Some(ids[index])
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
        project = options.project.as_str(),
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
    let Some(toolchain_id) = project.toolchains.first() else {
        return Ok("scratch".into());
    };

    let toolchain_registry = session.get_toolchain_registry().await?;

    if let Ok(toolchain) = toolchain_registry.load(&toolchain_id).await {
        if toolchain.has_func("define_docker_metadata").await {
            let metadata = toolchain
                .define_docker_metadata(DefineDockerMetadataInput {
                    context: toolchain_registry.create_context(),
                    toolchain_config: toolchain_registry.create_merged_config(
                        toolchain_id,
                        &session.toolchain_config,
                        &project.config,
                    ),
                })
                .await?;

            if let Some(image) = metadata.default_image {
                return Ok(image);
            }
        }
    }

    let image = match toolchain_id.as_str() {
        "bun" => "oven/bun:latest",
        "deno" => "denoland/deno:latest",
        "node" => "node:latest",
        "python" => "python:latest",
        "rust" => "rust:latest",
        _ => "scratch",
    };

    Ok(image.into())
}
