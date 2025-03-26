use super::dto::{GraphEdgeDto, GraphInfoDto, GraphNodeDto};
use miette::IntoDiagnostic;
use moon_action_graph::ActionGraph;
use moon_common::color;
use moon_env_var::GlobalEnvBag;
use moon_process::ProcessRegistry;
use moon_project_graph::{GraphConversions, ProjectGraph};
use moon_task_graph::TaskGraph;
use petgraph::{Graph, graph::NodeIndex};
use rustc_hash::FxHashMap;
use serde::Serialize;
use starbase_utils::json;
use std::fmt::Display;
use std::sync::Arc;
use tera::{Context, Tera};
use tiny_http::{Header, Request, Response, Server};
use tokio::task::{JoinHandle, spawn};

const INDEX_HTML: &str = include_str!("graph.html.tera");

#[derive(Debug, Serialize)]
pub struct RenderContext {
    pub page_title: String,
    pub graph_data: String,
    pub js_url: String,
}

pub async fn setup_server() -> miette::Result<(Arc<Server>, Tera)> {
    let bag = GlobalEnvBag::instance();

    let port = match bag.get("MOON_PORT") {
        Some(p) => p.parse::<u16>().unwrap(),
        None => 0, // Uses an available port
    };
    let host = match bag.get("MOON_HOST") {
        Some(h) => h,
        None => "127.0.0.1".to_string(),
    };
    let address = format!("{host}:{port}");
    let server = Server::http(address).unwrap();
    let tera = Tera::default();

    Ok((Arc::new(server), tera))
}

pub fn extract_nodes_and_edges_from_graph<T: Display>(
    graph: &Graph<String, T>,
    include_orphans: bool,
) -> GraphInfoDto {
    let mut nodes = FxHashMap::default();
    let edges = graph
        .raw_edges()
        .iter()
        .map(|e| GraphEdgeDto {
            label: e.weight.to_string(),
            source: e.source().index(),
            target: e.target().index(),
            id: format!("{} -> {}", e.source().index(), e.target().index()),
        })
        .collect::<Vec<_>>();

    let get_graph_node = |ni: NodeIndex| GraphNodeDto {
        id: ni.index(),
        label: graph
            .node_weight(ni)
            .expect("Unable to get node weight")
            .clone(),
    };

    for edge in graph.raw_edges().iter() {
        nodes.insert(edge.source(), get_graph_node(edge.source()));
        nodes.insert(edge.target(), get_graph_node(edge.target()));
    }

    if include_orphans {
        for ni in graph.node_indices() {
            nodes.entry(ni).or_insert_with(|| get_graph_node(ni));
        }
    }

    let nodes = nodes.into_values().collect();
    GraphInfoDto { edges, nodes }
}

/// Get a serialized representation of the project graph.
pub async fn project_graph_repr(project_graph: &ProjectGraph) -> GraphInfoDto {
    let labeled_graph = project_graph.to_labeled_graph();
    extract_nodes_and_edges_from_graph(&labeled_graph, true)
}

/// Get a serialized representation of the task graph.
pub async fn task_graph_repr(task_graph: &TaskGraph) -> GraphInfoDto {
    let labeled_graph = task_graph.to_labeled_graph();
    extract_nodes_and_edges_from_graph(&labeled_graph, true)
}

/// Get a serialized representation of the dependency graph.
pub async fn action_graph_repr(action_graph: &ActionGraph) -> GraphInfoDto {
    let labeled_graph = action_graph.labeled_graph();
    extract_nodes_and_edges_from_graph(&labeled_graph, false)
}

pub fn respond_to_request(
    req: Request,
    tera: &mut Tera,
    graph: &GraphInfoDto,
    page_title: String,
) -> miette::Result<()> {
    let response = match req.url() {
        "/graph-data" => {
            let mut response = Response::from_data(json::format(graph, false)?);
            response.add_header(
                Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap(),
            );
            response
        }
        _ => {
            let graph_data = json::format(graph, false)?;
            let js_url = get_js_url();
            let context = RenderContext {
                page_title,
                graph_data,
                js_url,
            };
            let info = tera
                .render_str(
                    INDEX_HTML,
                    &Context::from_serialize(context).into_diagnostic()?,
                )
                .unwrap();
            let mut response = Response::from_data(info);
            response
                .add_header(Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..]).unwrap());
            response
        }
    };

    req.respond(response).unwrap_or_default();

    Ok(())
}

// Use the local version of the JS file when in development mode otherwise the CDN URL.
pub fn get_js_url() -> String {
    match GlobalEnvBag::instance().get("MOON_JS_URL") {
        Some(url) => url,
        None => match cfg!(debug_assertions) {
            true => "http://localhost:5000/assets/index.js".to_string(),
            false => "https://unpkg.com/@moonrepo/visualizer@latest".to_string(),
        },
    }
}

pub async fn run_server(title: &str, graph_info: GraphInfoDto) -> miette::Result<()> {
    let (server, mut tera) = setup_server().await?;
    let url = format!("http://{}", server.server_addr());
    let _ = open::that(&url);

    println!("Started server on {}", color::url(url));

    let server_clone = server.clone();
    let handle1: JoinHandle<miette::Result<()>> = spawn(async move {
        let mut listener = ProcessRegistry::instance().receive_signal();

        if listener.recv().await.is_ok() {
            server_clone.unblock();
        }

        Ok(())
    });

    let title = title.to_owned();
    let handle2: JoinHandle<miette::Result<()>> = spawn(async move {
        for req in server.incoming_requests() {
            respond_to_request(req, &mut tera, &graph_info, title.clone())?;
        }

        Ok(())
    });

    tokio::try_join!(flatten(handle1), flatten(handle2))?;

    Ok(())
}

async fn flatten(handle: JoinHandle<miette::Result<()>>) -> miette::Result<()> {
    match handle.await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(err)) => Err(err),
        Err(err) => Err(miette::miette!("{err}")),
    }
}
