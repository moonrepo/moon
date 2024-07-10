use crate::session::CliSession;
use clap::Args;
use moon_common::Id;
use moon_config::PlatformType;
use moon_console::prompts::{Select, Text};
use moon_docker::*;
use starbase::AppResult;
use starbase_utils::fs;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct DockerFileArgs {
    #[arg(help = "ID of project to create a Dockerfile for")]
    id: Id,

    #[arg(long, help = "Use default value instead of prompting")]
    pub defaults: bool,

    #[arg(long, help = "ID of a task to build the project")]
    build_task: Option<Id>,

    #[arg(long, help = "Base Docker image")]
    image: Option<String>,

    #[arg(long, help = "ID of a task to run the project")]
    start_task: Option<Id>,
}

#[instrument(skip_all)]
pub async fn file(session: CliSession, args: DockerFileArgs) -> AppResult {
    let console = &session.console;
    let project_graph = session.get_project_graph().await?;

    // Ensure the project exists
    let project = project_graph.get(&args.id)?;

    // Build the options
    let mut options = GenerateDockerfileOptions {
        project: args.id,
        ..GenerateDockerfileOptions::default()
    };

    if let Some(image) = args.image {
        options.image = image;
    } else if args.defaults {
        options.image = project.config.docker.file.image.clone().unwrap_or_default();
    } else {
        options.image = console.prompt_text(
            Text::new("Docker image?").with_default(
                project
                    .config
                    .docker
                    .file
                    .image
                    .as_deref()
                    .unwrap_or_else(|| get_base_image_from_platform(&project.platform)),
            ),
        )?;
    }

    let build_task_id = if let Some(id) = &args.build_task {
        Some(id)
    } else if args.defaults {
        project.config.docker.file.build_task.as_ref()
    } else {
        let mut ids = project.tasks.keys().collect::<Vec<_>>();
        ids.sort();

        let starting_cursor = project
            .config
            .docker
            .file
            .build_task
            .as_ref()
            .and_then(|id| ids.iter().position(|cursor_id| cursor_id == &id));

        console.prompt_select_skippable(
            Select::new("Build task?", ids)
                .with_help_message("Skip build with ESC")
                .with_starting_cursor(starting_cursor.unwrap_or(0)),
        )?
    };

    if let Some(task_id) = build_task_id {
        options.build_task = Some(project.get_task(task_id)?.target.to_owned());
    }

    let start_task_id = if let Some(id) = &args.start_task {
        Some(id)
    } else if args.defaults {
        project.config.docker.file.start_task.as_ref()
    } else {
        let mut ids = project.tasks.keys().collect::<Vec<_>>();
        ids.sort();

        let starting_cursor = project
            .config
            .docker
            .file
            .start_task
            .as_ref()
            .and_then(|id| ids.iter().position(|cursor_id| cursor_id == &id));

        console.prompt_select_skippable(
            Select::new("Start task?", ids)
                .with_help_message("Skip start with ESC")
                .with_starting_cursor(starting_cursor.unwrap_or(0)),
        )?
    };

    if let Some(task_id) = start_task_id {
        options.start_task = Some(project.get_task(task_id)?.target.to_owned());
    }

    // Generate the file
    fs::write_file(
        project.root.join("Dockerfile"),
        generate_dockerfile(options)?,
    )?;

    Ok(())
}

fn get_base_image_from_platform(platform: &PlatformType) -> &str {
    match platform {
        PlatformType::Bun => "oven/bun:latest",
        PlatformType::Deno => "denoland/deno:latest",
        PlatformType::Node => "node:latest",
        PlatformType::Rust => "rust:latest",
        _ => "scratch",
    }
}
