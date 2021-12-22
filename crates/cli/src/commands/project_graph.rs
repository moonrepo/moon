use dot_writer::{ArrowType, Attributes, Color, DotWriter, Scope, Shape, Style};
use monolith_project::ROOT_NODE_ID;
use monolith_workspace::Workspace;

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
        create_node(dot_graph, id, false);

        // Map edges to node
        for dep in projects.get(id).unwrap().get_dependencies() {
            if dep != ROOT_NODE_ID {
                create_edge(dot_graph, &dep, id);
            }
        }
    }
}

fn graph_for_single_project(workspace: &Workspace, dot_graph: &mut Scope, id: &str) {
    workspace.projects.get(id).unwrap();

    // Add node to the graph
    create_node(dot_graph, id, true);

    for dep in workspace
        .projects
        .graph
        .borrow()
        .dependencies_of(&id.to_owned())
        .unwrap()
    {
        let dep_id = dep.unwrap();

        if dep_id != ROOT_NODE_ID && dep_id != id {
            create_node(dot_graph, dep_id, false);
            create_edge(dot_graph, id, dep_id);
        }
    }
}

pub async fn project_graph(workspace: &Workspace, id: &Option<String>) -> Result<(), clap::Error> {
    let mut output_bytes = Vec::new();

    {
        let mut writer = DotWriter::from(&mut output_bytes);

        writer.set_pretty_print(true);

        let mut dot_graph = writer.digraph();

        dot_graph
            .node_named(ROOT_NODE_ID)
            .set_style(Style::Filled)
            .set_shape(Shape::Circle)
            .set_fill_color(Color::Black)
            .set_font_color(Color::White);

        if let Some(project_id) = id {
            graph_for_single_project(workspace, &mut dot_graph, project_id);
        } else {
            graph_for_all_projects(workspace, &mut dot_graph);
        }
    }

    println!("{}", String::from_utf8(output_bytes).unwrap());

    Ok(())
}
