use crate::helpers::map_list;
use clap::Args;
use itertools::Itertools;
use miette::IntoDiagnostic;
use moon::build_project_graph;
use moon_app_components::Console;
use moon_common::Id;
use moon_utils::is_test_env;
use moon_workspace::Workspace;
use starbase::system;
use starbase_styles::color;

#[derive(Args, Clone, Debug)]
pub struct ProjectArgs {
    #[arg(help = "ID of project to display")]
    id: Id,

    #[arg(long, help = "Print in JSON format")]
    json: bool,
}

#[system]
pub async fn project(args: Args<ProjectArgs>, resources: Resources) {
    let mut workspace = resources.get::<Workspace>().await;
    let console = resources.get::<Console>().await;

    let project_graph = {
        let mut project_graph_builder = build_project_graph(&mut workspace).await?;
        project_graph_builder.load(&args.id).await?;
        project_graph_builder.build().await?
    };
    let project = project_graph.get(&args.id)?;
    let config = &project.config;

    let printer = console.stdout();

    if args.json {
        printer.write_line(serde_json::to_string_pretty(&project).into_diagnostic()?)?;

        return Ok(());
    }

    printer.print_header(&project.id)?;

    if let Some(meta) = &config.project {
        let mut has_other_meta = false;

        printer.write_line(&meta.description)?;
        printer.write_newline()?;

        if let Some(name) = &meta.name {
            printer.print_entry("Name", name)?;
            has_other_meta = true;
        }

        if let Some(owner) = &meta.owner {
            printer.print_entry("Owner", owner)?;
            has_other_meta = true;
        }

        if !meta.maintainers.is_empty() {
            printer.print_entry_list("Maintainers", &meta.maintainers)?;
            has_other_meta = true;
        }

        if let Some(channel) = &meta.channel {
            printer.print_entry("Channel", channel)?;
            has_other_meta = true;
        }

        if has_other_meta {
            printer.write_newline()?;
        }
    }

    printer.print_entry("Project", color::id(&project.id))?;

    if let Some(alias) = &project.alias {
        printer.print_entry("Alias", color::label(alias))?;
    }

    printer.print_entry("Source", color::file(&project.source))?;

    // Dont show in test snapshots
    if !is_test_env() {
        printer.print_entry("Root", color::path(&project.root))?;
    }

    if project.platform.is_javascript() {
        printer.print_entry("Platform", format!("{}", &project.platform))?;
    }

    printer.print_entry("Language", format!("{}", &project.language))?;
    printer.print_entry("Stack", format!("{}", &project.stack))?;
    printer.print_entry("Type", format!("{}", &project.type_of))?;

    if !config.tags.is_empty() {
        printer.print_entry("Tags", map_list(&config.tags, |tag| color::id(tag)))?;
    }

    let mut deps = vec![];

    for dep_config in &project.dependencies {
        deps.push(format!(
            "{} {}",
            color::id(&dep_config.id),
            color::muted(format!("({}, {})", dep_config.source, dep_config.scope)),
        ));
    }

    if !deps.is_empty() {
        deps.sort();

        printer.print_entry_header("Depends on")?;
        printer.print_list(deps)?;
    }

    if let Some(inherited) = &project.inherited {
        if !inherited.layers.is_empty() {
            let mut configs = vec![];

            for layer in inherited.layers.keys() {
                configs.push(color::file(layer));
            }

            printer.print_entry_header("Inherits from")?;
            printer.print_list(configs)?;
        }
    }

    if !project.tasks.is_empty() {
        printer.print_entry_header("Tasks")?;

        for name in project.tasks.keys().sorted() {
            let task = project.tasks.get(name).unwrap();

            if task.is_internal() {
                continue;
            }

            printer.print_entry(name, "")?;

            printer.write_line(format!(
                "  {} {}",
                color::muted("›"),
                color::shell(format!("{} {}", task.command, task.args.join(" "))),
            ))?;

            if let Some(description) = &task.description {
                printer.write_line(format!("    {description}"))?;
            }
        }
    }

    if !project.file_groups.is_empty() {
        printer.print_entry_header("File groups")?;

        for group_name in project.file_groups.keys().sorted() {
            let mut files = vec![];
            let group = project.file_groups.get(group_name).unwrap();

            for file in &group.files {
                files.push(color::file(file));
            }

            for file in &group.globs {
                files.push(color::file(file));
            }

            printer.print_entry_list(group_name, files)?;
        }
    }

    printer.write_newline()?;
    printer.flush()?;
}
