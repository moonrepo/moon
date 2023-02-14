use super::dto::{GraphEdgeDto, GraphInfoDto, GraphNodeDto};
use crate::helpers::AnyError;
use moon_dep_graph::DepGraph;
use moon_project_graph::ProjectGraph;
use petgraph::{graph::NodeIndex, Graph};
use rustc_hash::FxHashMap;
use serde::Serialize;
use std::env;
use tera::{Context, Tera};
use tiny_http::{Header, Request, Response, Server};

const INDEX_HTML: &str = include_str!("graph.html.tera");

#[derive(Debug, Serialize)]
pub struct RenderContext {
    pub page_title: String,

    pub graph_data: String,

    pub js_url: String,
}

pub async fn setup_server() -> Result<(Server, Tera), AnyError> {
    let port = match env::var("MOON_PORT") {
        Ok(p) => p.parse::<u16>().unwrap(),
        Err(..) => 8000,
    };
    let host = match env::var("MOON_HOST") {
        Ok(h) => h,
        Err(..) => "127.0.0.1".to_string(),
    };
    let address = format!("{host}:{port}");
    let server = Server::http(address).unwrap();
    let tera = Tera::default();

    Ok((server, tera))
}

pub fn extract_nodes_and_edges_from_graph(
    graph: &Graph<String, ()>,
    include_orphans: bool,
) -> GraphInfoDto {
    let mut nodes = FxHashMap::default();
    let edges = graph
        .raw_edges()
        .iter()
        .map(|e| GraphEdgeDto {
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
    let labeled_graph = project_graph.labeled_graph();
    extract_nodes_and_edges_from_graph(&labeled_graph, true)
}

/// Get a serialized representation of the dependency graph.
pub async fn dep_graph_repr(dep_graph: &DepGraph) -> GraphInfoDto {
    let labeled_graph = dep_graph.labeled_graph();
    extract_nodes_and_edges_from_graph(&labeled_graph, false)
}

pub fn respond_to_request(
    req: Request,
    tera: &mut Tera,
    graph: &GraphInfoDto,
    page_title: String,
) -> Result<(), AnyError> {
    let response = match req.url() {
        "/graph-data" => {
            let mut response = Response::from_data(serde_json::to_string(graph)?);
            response.add_header(
                Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap(),
            );
            response
        }
        _ => {
            let graph_data = serde_json::to_string(graph)?;
            let js_url = get_js_url();
            let context = RenderContext {
                page_title,
                graph_data,
                js_url,
            };
            let info = tera
                .render_str(INDEX_HTML, &Context::from_serialize(&context)?)
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
    match env::var("MOON_JS_URL") {
        Ok(url) => url,
        Err(..) => match cfg!(debug_assertions) {
            true => "http://localhost:5000/assets/index.js".to_string(),
            false => "https://unpkg.com/@moonrepo/visualizer@latest".to_string(),
        },
    }
}
