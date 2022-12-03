use super::dto::{GraphEdgeDto, GraphInfoDto, GraphNodeDto};
use crate::helpers::AnyError;
use moon_dep_graph::DepGraph;
use moon_project_graph::ProjectGraph;
use petgraph::{graph::NodeIndex, Graph};
use rustc_hash::FxHashSet;
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
    let address = format!("{}:{}", host, port);
    let server = Server::http(address).unwrap();
    let tera = Tera::default();

    Ok((server, tera))
}

pub fn extract_nodes_and_edges_from_graph(graph: &Graph<String, ()>) -> GraphInfoDto {
    let edges = graph
        .raw_edges()
        .iter()
        .map(|e| GraphEdgeDto {
            source: e.source().index(),
            target: e.target().index(),
            id: format!("{}->{}", e.source().index(), e.target().index()),
        })
        .collect::<Vec<_>>();
    let mut nodes = FxHashSet::default();
    for edge in graph.raw_edges().iter() {
        let get_graph_node = |ni: NodeIndex| GraphNodeDto {
            id: ni.index(),
            label: graph
                .node_weight(ni)
                .expect("Unable to get node weight")
                .clone(),
        };
        nodes.insert(get_graph_node(edge.source()));
        nodes.insert(get_graph_node(edge.target()));
    }
    let nodes = nodes.into_iter().collect();
    GraphInfoDto { edges, nodes }
}

/// Get a serialized representation of the project graph.
pub async fn project_graph_repr(project_graph: &ProjectGraph) -> GraphInfoDto {
    let labeled_graph = project_graph.labeled_graph();
    extract_nodes_and_edges_from_graph(&labeled_graph)
}

/// Get a serialized representation of the dependency graph.
pub async fn dep_graph_repr(dep_graph: &DepGraph) -> GraphInfoDto {
    let labeled_graph = dep_graph.labeled_graph();
    extract_nodes_and_edges_from_graph(&labeled_graph)
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
            // Use the local version of the JS file when in development mode otherwise the
            // CDN url.
            let js_url = match cfg!(debug_assertions) {
                true => get_js_url(false),
                false => get_js_url(true),
            };
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

pub fn get_js_url(is_production: bool) -> String {
    match env::var("MOON_JS_URL") {
        Ok(url) => url,
        Err(..) => match is_production {
            false => "http://localhost:5000/assets/index.js".to_string(),
            true => "https://unpkg.com/@moonrepo/visualizer@latest".to_string(),
        },
    }
}
