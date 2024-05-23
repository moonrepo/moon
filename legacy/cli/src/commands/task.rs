use clap::Args;
use miette::{miette, IntoDiagnostic};
use moon::build_project_graph;
use moon_app_components::Console;
use moon_target::{Target, TargetScope};
use moon_workspace::Workspace;
use starbase::system;
use starbase_styles::color;

#[derive(Args, Clone, Debug)]
pub struct TaskArgs {
    #[arg(help = "Target of task to display")]
    target: Target,

    #[arg(long, help = "Print in JSON format")]
    json: bool,
}

#[system]
pub async fn task(args: ArgsRef<TaskArgs>, resources: Resources) {
    let TargetScope::Project(project_locator) = &args.target.scope else {
        return Err(miette!(code = "moon::task", "A project ID is required."));
    };

    let mut workspace = resources.get_async::<Workspace>().await;
    let console = resources.get_async::<Console>().await;

    let project_graph = {
        let mut project_graph_builder = build_project_graph(&mut workspace).await?;
        project_graph_builder.load(project_locator).await?;
        project_graph_builder.build().await?
    };
    let project = project_graph.get(project_locator)?;
    let task = project.get_task(&args.target.task_id)?;

    let printer = console.stdout();

    if args.json {
        printer.write_line(serde_json::to_string_pretty(&task).into_diagnostic()?)?;

        return Ok(());
    }

    printer.print_header(&args.target.id)?;

    if let Some(desc) = &task.description {
        printer.write_line(desc)?;
        printer.write_newline()?;
    }

    printer.print_entry("Task", color::id(&args.target.task_id))?;
    printer.print_entry("Project", color::id(&project.id))?;
    printer.print_entry("Platform", format!("{}", &task.platform))?;
    printer.print_entry("Type", format!("{}", &task.type_of))?;

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
        printer.print_entry("Modes", modes.join(", "))?;
    }

    printer.print_entry_header("Process")?;
    printer.print_entry(
        "Command",
        color::shell(format!("{} {}", task.command, task.args.join(" "))),
    )?;

    if !task.env.is_empty() {
        printer.print_entry_list(
            "Environment variables",
            task.env
                .iter()
                .map(|(k, v)| format!("{} {} {}", k, color::muted_light("="), v))
                .collect::<Vec<_>>(),
        )?;
    }

    printer.print_entry(
        "Working directory",
        color::path(if task.options.run_from_workspace_root {
            &workspace.root
        } else {
            &project.root
        }),
    )?;
    printer.print_entry(
        "Runs dependencies",
        if task.options.run_deps_in_parallel {
            "Concurrently"
        } else {
            "Serially"
        },
    )?;
    printer.print_entry_bool("Runs in CI", task.should_run_in_ci())?;

    if !task.deps.is_empty() {
        printer.print_entry_header("Depends on")?;
        printer.print_list(
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
                printer.print_entry_header("Inherits from")?;
                printer.print_list(task_layers.iter().map(color::file).collect::<Vec<_>>())?;
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

        printer.print_entry_header("Inputs")?;
        printer.print_list(files)?;
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

        printer.print_entry_header("Outputs")?;
        printer.print_list(files)?;
    }

    printer.write_newline()?;
    printer.flush()?;
}
