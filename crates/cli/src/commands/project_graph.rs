use moon_workspace::Workspace;

pub async fn project_graph(id: &Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = Workspace::load().await?;

    // Force load projects into the graph
    if let Some(pid) = id {
        workspace.projects.load(pid)?;
    } else {
        for pid in workspace.projects.ids() {
            workspace.projects.load(&pid)?;
        }
    }

    println!("{}", workspace.projects.to_dot());

    Ok(())
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    use crate::helpers::{create_test_command, get_assert_output};

    #[test]
    fn no_projects() {
        let assert = create_test_command("base").arg("project-graph").assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn many_projects() {
        let assert = create_test_command("projects")
            .arg("project-graph")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn single_project_with_dependencies() {
        let assert = create_test_command("projects")
            .arg("project-graph")
            .arg("foo")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn single_project_no_dependencies() {
        let assert = create_test_command("projects")
            .arg("project-graph")
            .arg("baz")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }
}
