use crate::session::CliSession;
use clap::Args;
use moon_common::{color, Id};
use moon_config::PlatformType;
use moon_console::prompts::{Select, Text};
use moon_docker::*;
use starbase::AppResult;
use starbase_utils::fs;
use tracing::{debug, instrument};

#[derive(Args, Clone, Debug)]
pub struct DockerFileArgs {
    #[arg(help = "ID of project to create a Dockerfile for")]
    id: Id,

    #[arg(long, help = "Use default options instead of prompting")]
    defaults: bool,

    #[arg(help = "Destination path, relative from the project root")]
    dest: Option<String>,

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
pub async fn file(session: CliSession, args: DockerFileArgs) -> AppResult {
    let console = &session.console;
    let project_graph = session.get_project_graph().await?;

    // Ensure the project exists
    let project = project_graph.get(&args.id)?;

    // Build the options
    let mut options = GenerateDockerfileOptions {
        disable_toolchain: args.no_toolchain,
        project: args.id,
        prune: !args.no_prune,
        ..GenerateDockerfileOptions::default()
    };

    debug!("Gathering Dockerfile options");

    if let Some(image) = args.image {
        options.image = image;
    } else if args.defaults {
        options.image = project
            .config
            .docker
            .file
            .image
            .clone()
            .unwrap_or_else(|| get_base_image_from_platform(&project.platform).into());
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

    debug!(image = &options.image, "Using Docker image");

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
        let target = project.get_task(task_id)?.target.to_owned();

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
        let target = project.get_task(task_id)?.target.to_owned();

        debug!(task = target.as_str(), "Using start task");

        options.start_task = Some(target);
    } else {
        debug!("Not using a start task");
    }

    // Generate the file
    let out = args.dest.unwrap_or("Dockerfile".into());
    let out_file = project.root.join(&out);

    debug!(
        dockerfile = ?out_file,
        project = options.project.as_str(),
        "Generating Dockerfile in project",
    );

    fs::write_file(out_file, generate_dockerfile(options)?)?;

    console.out.write_line(format!(
        "Generated {}",
        color::rel_path(project.source.join(out))
    ))?;

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
