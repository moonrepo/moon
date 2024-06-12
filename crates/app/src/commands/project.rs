use crate::session::CliSession;
use clap::Args;
use moon_common::{is_test_env, Id};
use starbase::AppResult;
use starbase_styles::color;
use starbase_utils::json;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct ProjectArgs {
    #[arg(help = "ID of project to display")]
    id: Id,

    #[arg(long, help = "Print in JSON format")]
    json: bool,
}

#[instrument(skip_all)]
pub async fn project(session: CliSession, args: ProjectArgs) -> AppResult {
    let mut project_graph_builder = session.build_project_graph().await?;
    project_graph_builder.load(&args.id).await?;

    let project_graph = project_graph_builder.build().await?;
    let project = project_graph.get(&args.id)?;
    let config = &project.config;

    let console = session.console.stdout();

    if args.json {
        console.write_line(json::format(&project, true)?)?;

        return Ok(());
    }

    console.print_header(&project.id)?;

    if let Some(meta) = &config.project {
        let mut has_other_meta = false;

        console.write_line(&meta.description)?;
        console.write_newline()?;

        if let Some(name) = &meta.name {
            console.print_entry("Name", name)?;
            has_other_meta = true;
        }

        if let Some(owner) = &meta.owner {
            console.print_entry("Owner", owner)?;
            has_other_meta = true;
        }

        if !meta.maintainers.is_empty() {
            console.print_entry_list("Maintainers", &meta.maintainers)?;
            has_other_meta = true;
        }

        if let Some(channel) = &meta.channel {
            console.print_entry("Channel", channel)?;
            has_other_meta = true;
        }

        if has_other_meta {
            console.write_newline()?;
        }
    }

    console.print_entry("Project", color::id(&project.id))?;

    if let Some(alias) = &project.alias {
        console.print_entry("Alias", color::label(alias))?;
    }

    console.print_entry("Source", color::file(&project.source))?;

    // Dont show in test snapshots
    if !is_test_env() {
        console.print_entry("Root", color::path(&project.root))?;
    }

    if project.platform.is_javascript() {
        console.print_entry("Platform", format!("{}", &project.platform))?;
    }

    console.print_entry("Language", format!("{}", &project.language))?;
    console.print_entry("Stack", format!("{}", &project.stack))?;
    console.print_entry("Type", format!("{}", &project.type_of))?;

    if !config.tags.is_empty() {
        console.print_entry(
            "Tags",
            config
                .tags
                .iter()
                .map(color::id)
                .collect::<Vec<_>>()
                .join(", "),
        )?;
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

        console.print_entry_header("Depends on")?;
        console.print_list(deps)?;
    }

    if let Some(inherited) = &project.inherited {
        if !inherited.layers.is_empty() {
            let mut configs = vec![];

            for layer in inherited.layers.keys() {
                configs.push(color::file(layer));
            }

            console.print_entry_header("Inherits from")?;
            console.print_list(configs)?;
        }
    }

    if !project.tasks.is_empty() {
        console.print_entry_header("Tasks")?;

        for name in project.tasks.keys() {
            let task = project.tasks.get(name).unwrap();

            if task.is_internal() {
                continue;
            }

            console.print_entry(name, "")?;

            console.write_line(format!(
                "  {} {}",
                color::muted("›"),
                color::shell(format!("{} {}", task.command, task.args.join(" "))),
            ))?;

            if let Some(description) = &task.description {
                console.write_line(format!("    {description}"))?;
            }
        }
    }

    if !project.file_groups.is_empty() {
        console.print_entry_header("File groups")?;

        for group_name in project.file_groups.keys() {
            let mut files = vec![];
            let group = project.file_groups.get(group_name).unwrap();

            for file in &group.files {
                files.push(color::file(file));
            }

            for file in &group.globs {
                files.push(color::file(file));
            }

            console.print_entry_list(group_name, files)?;
        }
    }

    console.write_newline()?;
    console.flush()?;

    Ok(())
}
