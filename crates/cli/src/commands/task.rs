use crate::helpers::AnyError;
use console::Term;
use moon::{build_project_graph, load_workspace};
use moon_target::Target;
use moon_terminal::{ExtendedTerm, Label};
use starbase_styles::color;

pub async fn task(id: String, json: bool) -> Result<(), AnyError> {
    let target = Target::parse(&id)?;

    let Some(project_id) = target.scope_id else {
      return Err("A project ID is required.".into());
    };

    let mut workspace = load_workspace().await?;
    let mut project_builder = build_project_graph(&mut workspace).await?;
    project_builder.load(&project_id)?;

    let project_graph = project_builder.build()?;
    let project = project_graph.get(&project_id)?;
    let task = project.get_task(&target.task_id)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&task)?);

        return Ok(());
    }

    let term = Term::buffered_stdout();

    term.write_line("")?;
    term.render_label(Label::Brand, &target.id)?;
    term.render_entry("Task", color::id(&target.task_id))?;
    term.render_entry("Project", color::id(&project_id))?;
    term.render_entry("Platform", term.format(&task.platform))?;
    term.render_entry("Type", term.format(&task.type_of))?;

    term.write_line("")?;
    term.render_label(Label::Default, "Process")?;
    term.render_entry(
        "Command",
        color::shell(format!("{} {}", task.command, task.args.join(" "))),
    )?;

    if !task.env.is_empty() {
        term.render_entry_list(
            "Environment variables",
            task.env
                .iter()
                .map(|(k, v)| format!("{} {} {}", k, color::muted_light("="), v))
                .collect::<Vec<_>>(),
        )?;
    }

    term.render_entry(
        "Working directory",
        color::path(if task.options.run_from_workspace_root {
            &workspace.root
        } else {
            &project.root
        }),
    )?;
    term.render_entry(
        "Runs dependencies",
        if task.options.run_deps_in_parallel {
            "Concurrently"
        } else {
            "Serially"
        },
    )?;
    term.render_entry_bool("Runs in CI", task.options.run_in_ci)?;

    if !task.deps.is_empty() {
        term.write_line("")?;
        term.render_label(Label::Default, "Depends on")?;
        term.render_list(task.deps.iter().map(color::label).collect::<Vec<_>>())?;
    }

    if !task.input_paths.is_empty() || !task.input_globs.is_empty() {
        term.write_line("")?;
        term.render_label(Label::Default, "Inputs")?;
        term.render_list(task.input_globs.iter().map(color::file).collect::<Vec<_>>())?;
        term.render_list(task.input_paths.iter().map(color::path).collect::<Vec<_>>())?;
    }

    if !task.output_paths.is_empty() || !task.output_globs.is_empty() {
        term.write_line("")?;
        term.render_label(Label::Default, "Outputs")?;
        term.render_list(
            task.output_globs
                .iter()
                .map(color::file)
                .collect::<Vec<_>>(),
        )?;
        term.render_list(
            task.output_paths
                .iter()
                .map(color::path)
                .collect::<Vec<_>>(),
        )?;
    }

    term.write_line("")?;
    term.flush()?;

    Ok(())
}
