use crate::terminal::{color, ExtendedTerm, Label};
use console::Term;
use itertools::Itertools;
use moon_workspace::Workspace;

pub async fn project(id: &str, json: &bool) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = Workspace::load()?;
    let project = workspace.projects.get(id)?;

    if *json {
        println!("{}", project.to_json());

        return Ok(());
    }

    let term = Term::buffered_stdout();

    term.write_line("")?;
    term.render_label(Label::Brand, &project.id)?;
    term.render_entry("ID", &color::id(&project.id))?;
    term.render_entry("Path", &color::path(&project.location))?;
    term.render_entry("Root", &color::file_path(&project.dir))?;

    if let Some(config) = project.config {
        if let Some(meta) = config.project {
            term.render_entry("Type", &term.format(&meta.type_of))?;
            term.render_entry("Name", &meta.name)?;
            term.render_entry("Description", &meta.description)?;
            term.render_entry("Owner", &meta.owner)?;
            term.render_entry_list("Maintainers", &meta.maintainers)?;
            term.render_entry("Channel", &meta.channel)?;
        }

        if let Some(depends_on) = config.depends_on {
            let mut deps = vec![];

            for dep_id in depends_on {
                match workspace.projects.get(&dep_id) {
                    Ok(dep) => {
                        deps.push(format!(
                            "{} {}{}{}",
                            color::id(&dep_id),
                            color::muted_light("("),
                            color::path(&dep.location),
                            color::muted_light(")"),
                        ));
                    }
                    Err(_) => {
                        deps.push(color::id(&dep_id));
                    }
                };
            }

            term.write_line("")?;
            term.render_label(Label::Default, "Depends on")?;
            term.render_list(&deps)?;
        }
    }

    if !project.tasks.is_empty() {
        term.write_line("")?;
        term.render_label(Label::Default, "Tasks")?;

        for name in project.tasks.keys().sorted() {
            let task = project.tasks.get(name).unwrap();

            term.render_entry(
                name,
                &color::shell(&format!("{} {}", task.command, task.args.join(" "))),
            )?;
        }
    }

    if !project.file_groups.is_empty() {
        term.write_line("")?;
        term.render_label(Label::Default, "File groups")?;

        for group in project.file_groups.keys().sorted() {
            let mut files = vec![];

            for file in project.file_groups.get(group).unwrap() {
                files.push(color::path(file));
            }

            term.render_entry_list(group, &files)?;
        }
    }

    term.write_line("")?;
    term.flush()?;

    Ok(())
}
