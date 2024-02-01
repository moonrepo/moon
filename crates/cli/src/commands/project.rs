use crate::helpers::map_list;
use clap::Args;
use itertools::Itertools;
use miette::IntoDiagnostic;
use moon::build_project_graph;
use moon_app_components::AppConsole;
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
pub async fn project(args: ArgsRef<ProjectArgs>, resources: ResourcesMut) {
    let mut project_graph_builder =
        { build_project_graph(resources.get_mut::<Workspace>()).await? };
    project_graph_builder.load(&args.id).await?;

    let project_graph = project_graph_builder.build().await?;
    let project = project_graph.get(&args.id)?;
    let config = &project.config;

    let console = resources.get::<AppConsole>().stdout();

    if args.json {
        console.write_line(serde_json::to_string_pretty(&project).into_diagnostic()?)?;

        return Ok(());
    }

    console.print_header(&project.id)?;
    console.print_entry("Project", color::id(&project.id))?;

    if let Some(alias) = &project.alias {
        console.print_entry("Alias", color::label(alias))?;
    }

    console.print_entry("Source", color::file(&project.source))?;

    // Dont show in test snapshots
    if !is_test_env() {
        console.print_entry("Root", color::path(&project.root))?;
    }

    console.print_entry("Language", format!("{}", &project.language))?;
    console.print_entry("Type", format!("{}", &project.type_of))?;

    if !config.tags.is_empty() {
        console.print_entry("Tags", map_list(&config.tags, |tag| color::id(tag)))?;
    }

    if let Some(meta) = &config.project {
        if let Some(name) = &meta.name {
            console.print_entry("Name", name)?;
        }

        console.print_entry("Description", &meta.description)?;

        if let Some(owner) = &meta.owner {
            console.print_entry("Owner", owner)?;
        }

        if !meta.maintainers.is_empty() {
            console.print_entry_list("Maintainers", &meta.maintainers)?;
        }

        if let Some(channel) = &meta.channel {
            console.print_entry("Channel", channel)?;
        }
    }

    let mut deps = vec![];

    for dep_config in &project.dependencies {
        deps.push(format!(
            "{} {}",
            color::id(&dep_config.id),
            color::muted_light(format!("({}, {})", dep_config.source, dep_config.scope)),
        ));
    }

    if !deps.is_empty() {
        deps.sort();

        console.print_entry_header("Depends on")?;
        console.print_list(deps)?;
    }

    if !project.tasks.is_empty() {
        console.print_entry_header("Tasks")?;

        for name in project.tasks.keys().sorted() {
            let task = project.tasks.get(name).unwrap();

            console.print_entry(
                name,
                color::shell(format!("{} {}", task.command, task.args.join(" "))),
            )?;
        }
    }

    if !project.file_groups.is_empty() {
        console.print_entry_header("File groups")?;

        for group_name in project.file_groups.keys().sorted() {
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
}
