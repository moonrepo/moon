use crate::app_error::AppError;
use crate::session::CliSession;
use clap::Args;
use moon_task::{Target, TargetScope};
use starbase::AppResult;
use starbase_styles::color;
use starbase_utils::json;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct TaskArgs {
    #[arg(help = "Target of task to display")]
    target: Target,

    #[arg(long, help = "Print in JSON format")]
    json: bool,
}

#[instrument(skip_all)]
pub async fn task(session: CliSession, args: TaskArgs) -> AppResult {
    let TargetScope::Project(project_locator) = &args.target.scope else {
        return Err(AppError::ProjectIdRequired.into());
    };

    let mut project_graph_builder = session.build_project_graph().await?;
    project_graph_builder.load(project_locator).await?;

    let project_graph = project_graph_builder.build().await?;
    let project = project_graph.get(project_locator)?;
    let task = project.get_task(&args.target.task_id)?;

    let console = session.console.stdout();

    if args.json {
        console.write_line(json::format(&task, true)?)?;

        return Ok(());
    }

    console.print_header(&args.target.id)?;

    if let Some(desc) = &task.description {
        console.write_line(desc)?;
        console.write_newline()?;
    }

    console.print_entry("Task", color::id(&args.target.task_id))?;
    console.print_entry("Project", color::id(&project.id))?;
    console.print_entry("Platform", format!("{}", &task.platform))?;
    console.print_entry("Type", format!("{}", &task.type_of))?;

    let mut modes = vec![];

    if task.is_local() {
        modes.push("local");
    }
    if task.is_internal() {
        modes.push("internal");
    }
    if task.is_interactive() {
        modes.push("interactive");
    }
    if task.is_persistent() {
        modes.push("persistent");
    }

    if !modes.is_empty() {
        console.print_entry("Modes", modes.join(", "))?;
    }

    console.print_entry_header("Process")?;
    console.print_entry(
        if task.script.is_some() {
            "Script"
        } else {
            "Command"
        },
        color::shell(task.get_command_line()),
    )?;

    if !task.env.is_empty() {
        console.print_entry_list(
            "Environment variables",
            task.env
                .iter()
                .map(|(k, v)| format!("{} {} {}", k, color::muted_light("="), v))
                .collect::<Vec<_>>(),
        )?;
    }

    console.print_entry(
        "Working directory",
        color::path(if task.options.run_from_workspace_root {
            &session.workspace_root
        } else {
            &project.root
        }),
    )?;
    console.print_entry(
        "Runs dependencies",
        if task.options.run_deps_in_parallel {
            "Concurrently"
        } else {
            "Serially"
        },
    )?;
    console.print_entry_bool("Runs in CI", task.should_run_in_ci())?;

    if !task.deps.is_empty() {
        console.print_entry_header("Depends on")?;
        console.print_list(
            task.deps
                .iter()
                .map(|d| color::label(&d.target))
                .collect::<Vec<_>>(),
        )?;
    }

    if let Some(inherited) = &project.inherited {
        if let Some(task_layers) = inherited.task_layers.get(task.id.as_str()) {
            if !task_layers.is_empty()
                && !project
                    .config
                    .workspace
                    .inherited_tasks
                    .exclude
                    .contains(&task.id)
            {
                console.print_entry_header("Inherits from")?;
                console.print_list(task_layers.iter().map(color::file).collect::<Vec<_>>())?;
            }
        }
    }

    if !task.input_files.is_empty() || !task.input_globs.is_empty() {
        let mut files = vec![];
        files.extend(
            task.input_globs
                .iter()
                .map(color::rel_path)
                .collect::<Vec<_>>(),
        );
        files.extend(
            task.input_files
                .iter()
                .map(color::rel_path)
                .collect::<Vec<_>>(),
        );

        console.print_entry_header("Inputs")?;
        console.print_list(files)?;
    }

    if !task.output_files.is_empty() || !task.output_globs.is_empty() {
        let mut files = vec![];
        files.extend(
            task.output_globs
                .iter()
                .map(color::rel_path)
                .collect::<Vec<_>>(),
        );
        files.extend(
            task.output_files
                .iter()
                .map(color::rel_path)
                .collect::<Vec<_>>(),
        );

        console.print_entry_header("Outputs")?;
        console.print_list(files)?;
    }

    console.write_newline()?;
    console.flush()?;

    Ok(())
}
