use crate::helpers::{print_list, safe_exit};
use itertools::Itertools;
use monolith_workspace::Workspace;

enum ProjectExitCodes {
    UnknownProject = 1,
}

pub async fn project(workspace: &Workspace, id: &str, json: &bool) -> Result<(), clap::Error> {
    let project = match workspace.projects.get(id) {
        Ok(data) => data,
        Err(_) => {
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

    if let Some(config) = project.config {
        if let Some(meta) = config.project {
            println!("Name: {}", meta.name);
            println!("Description: {}", meta.description);
            println!("Owner: {}", meta.owner);
            println!("Maintainers:");
            print_list(&meta.maintainers);
            println!("Channel: {}", meta.channel);
        }

        if let Some(depends_on) = config.depends_on {
            println!();
            println!("Depends on");

            for dep_id in depends_on {
                match workspace.projects.get(&dep_id) {
                    Ok(dep) => {
                        println!("- {} ({})", dep_id, dep.location);
                    }
                    Err(_) => {
                        println!("- {}", dep_id);
                    }
                }
            }
        }
    }

    if !project.file_groups.is_empty() {
        println!();
        println!("File groups");

        for group in project.file_groups.keys().sorted() {
            println!("{}:", group);
            print_list(project.file_groups.get(group).unwrap());
        }
    }

    Ok(())
}
