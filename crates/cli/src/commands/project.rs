use crate::helpers::{print_list, safe_exit};
use monolith_workspace::Workspace;

enum ProjectExitCodes {
    UnknownProject = 1,
}

pub async fn project(workspace: &Workspace, id: &str, json: &bool) -> Result<(), clap::Error> {
    let project = match workspace.projects.get(id) {
        Some(data) => data,
        None => {
            eprintln!("Project \"{}\" not found.", id);
            safe_exit(ProjectExitCodes::UnknownProject as i32);
        }
    };

    if *json {
        println!("{}", project.to_json());

        return Ok(());
    }

    println!("About");
    println!("ID: {}", project.id);
    println!("Path: {}", project.location);

    if project.config.is_some() {
        let config = project.config.as_ref().unwrap();

        if config.project.is_some() {
            let meta = config.project.as_ref().unwrap();

            println!("Name: {}", meta.name);
            println!("Description: {}", meta.description);
            println!("Owner: {}", meta.owner);
            println!("Maintainers:");
            print_list(&meta.maintainers);
            println!("Channel: {}", meta.channel);
        }

        if config.depends_on.is_some() {
            println!();
            println!("Depends on");
            print_list(config.depends_on.as_ref().unwrap());
        }
    }

    if !project.file_groups.is_empty() {
        println!();
        println!("File groups");

        for (group, globs) in &project.file_groups {
            println!("{}:", group);
            print_list(globs);
        }
    }

    Ok(())
}
