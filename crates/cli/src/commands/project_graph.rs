use dot_writer::{ArrowType, Attributes, Color, DotWriter, Scope, Shape, Style};
use moon_project::{ProjectError, ROOT_NODE_ID};
use moon_workspace::Workspace;

fn create_edge(dot_graph: &mut Scope, from: &str, to: &str) {
    let mut attr = dot_graph.edge(from, to).attributes();

    if from == ROOT_NODE_ID {
        attr.set_arrow_head(ArrowType::None);
    } else {
        attr.set_arrow_head(ArrowType::Box)
            .set_arrow_tail(ArrowType::Box);
    }
}

fn create_node(dot_graph: &mut Scope, id: &str, highlight: bool) {
    dot_graph
        .node_named(id)
        .set_style(Style::Filled)
        .set_shape(Shape::Circle)
        .set_fill_color(if highlight {
            Color::PaleGreen
        } else {
            Color::Grey
        })
        .set_font_color(Color::Black);

    // Map it to the root
    create_edge(dot_graph, ROOT_NODE_ID, id);
}

fn graph_for_all_projects(workspace: &Workspace, dot_graph: &mut Scope) {
    let projects = &workspace.projects;

    for id in projects.ids() {
        // Add node to the graph
        create_node(dot_graph, &id, false);

        // Load project and map deps as edges
        for dep in projects.get(&id).unwrap().get_dependencies() {
            create_edge(dot_graph, &dep, &id);
        }
    }
}

fn graph_for_single_project(
    workspace: &Workspace,
    dot_graph: &mut Scope,
    id: &str,
) -> Result<(), ProjectError> {
    // Load project
    let project = workspace.projects.get(id)?;

    // Add node to the graph
    create_node(dot_graph, id, true);

    // Map deps as edges
    for dep_id in workspace.projects.get_sorted_dependencies_of(&project)? {
        if dep_id != id {
            create_node(dot_graph, &dep_id, false);
            create_edge(dot_graph, id, &dep_id);
        }
    }

    Ok(())
}

pub async fn project_graph(id: &Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = Workspace::load()?;

    if let Some(pid) = id {
        workspace.projects.get(pid)?;
    } else {
        for pid in workspace.projects.ids() {
            workspace.projects.get(&pid)?;
        }
    }

    // let mut output_bytes = Vec::new();
    // {
    //     let mut writer = DotWriter::from(&mut output_bytes);

    //     writer.set_pretty_print(true);

    //     let mut dot_graph = writer.digraph();

    //     dot_graph
    //         .node_named(ROOT_NODE_ID)
    //         .set_style(Style::Filled)
    //         .set_shape(Shape::Circle)
    //         .set_fill_color(Color::Black)
    //         .set_font_color(Color::White);

    //     if let Some(project_id) = id {
    //         graph_for_single_project(&workspace, &mut dot_graph, project_id)?;
    //     } else {
    //         graph_for_all_projects(&workspace, &mut dot_graph);
    //     }
    // }

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
