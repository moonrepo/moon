use moon_project::{TaskGraph, TouchedFilePaths};
use moon_workspace::{TouchedFiles, Workspace};
use std::collections::HashSet;
// use std::fs;
use std::io;

// TODO: Filter touched files based on their last modified time
fn get_touched_files(
    workspace: &Workspace,
    touched_files: TouchedFiles,
) -> io::Result<TouchedFilePaths> {
    let mut affected = HashSet::new();

    for file in &touched_files.all {
        let path = workspace.dir.join(file);
        // let meta = fs::metadata(&path)?;

        // if let Ok(time) = meta.modified() {
        //     // TODO needs cache impl
        // } else {
        //     // Unable to get last modified time, so assume affected
        //     affected.insert(path);
        // }

        affected.insert(path);
    }

    Ok(affected)
}

pub async fn run(target: &str) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = Workspace::load()?;

    // Gather files that have been touched in the working tree
    let touched_files = get_touched_files(&workspace, workspace.vcs.get_touched_files().await?)?;

    // Generate a task graph, that filters projects and tasks based on affected files
    let _graph = TaskGraph::new(&workspace.projects, &touched_files, target.to_owned());

    Ok(())
}
