use crate::terminal::{color, ExtendedTerm, Label};
use console::Term;
use itertools::Itertools;
use moon_workspace::Workspace;
use std::env;

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

    // Dont show in test snapshots
    if env::var("MOON_TEST").is_err() {
        term.render_entry("Root", &color::file_path(&project.dir))?;
    }

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

            for file in &project.file_groups.get(group).unwrap().files {
                files.push(color::path(file));
            }

            term.render_entry_list(group, &files)?;
        }
    }

    term.write_line("")?;
    term.flush()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::helpers::{create_test_command, get_assert_output, get_assert_stderr_output};
    use insta::assert_snapshot;

    #[test]
    fn unknown_project() {
        let assert = create_test_command("projects")
            .arg("project")
            .arg("unknown")
            .assert();

        assert_snapshot!(get_assert_stderr_output(&assert));

        assert.failure().code(1);
    }

    #[test]
    fn empty_config() {
        let assert = create_test_command("projects")
            .arg("project")
            .arg("emptyConfig")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn no_config() {
        let assert = create_test_command("projects")
            .arg("project")
            .arg("noConfig")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn basic_config() {
        // with dependsOn and fileGroups
        let assert = create_test_command("projects")
            .arg("project")
            .arg("basic")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn advanced_config() {
        // with project metadata
        let assert = create_test_command("projects")
            .arg("project")
            .arg("advanced")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn depends_on_paths() {
        // shows dependsOn paths when they exist
        let assert = create_test_command("projects")
            .arg("project")
            .arg("foo")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn with_tasks() {
        let assert = create_test_command("projects")
            .arg("project")
            .arg("tasks")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }
}
