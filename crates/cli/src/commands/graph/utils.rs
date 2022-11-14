use super::dto::{GraphEdgeDto, GraphInfoDto, GraphNodeDto};
use crate::helpers::AnyError;
use moon_logger::info;
use moon_runner::DepGraph;
use moon_workspace::Workspace;
use petgraph::Graph;
use serde::Serialize;
use std::{collections::HashSet, env};
use tera::{Context, Tera};
use tiny_http::{Header, Response, Server};

const INDEX_HTML: &str = include_str!("graph.html.tera");
const LOG_TARGET: &str = "moon:graph::utils";

#[derive(Debug, Serialize)]
pub struct RenderContext {
    pub graph_data: String,

    pub js_url: String,
}

pub async fn setup_server() -> Result<(Server, Tera), AnyError> {
    let port = match env::var("PORT") {
        Ok(p) => p.parse::<u16>().unwrap(),
        Err(..) => 8000,
    };
    let host = match env::var("HOST") {
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
    let mut nodes = HashSet::new();
    for edge in graph.raw_edges().iter() {
        let source = graph
            .node_weight(edge.source())
            .expect("Unable to get node")
            .clone();
        let target = graph
            .node_weight(edge.target())
            .expect("Unable to get node")
            .clone();
        nodes.insert(GraphNodeDto {
            id: edge.source().index(),
            label: source,
        });
        nodes.insert(GraphNodeDto {
            id: edge.target().index(),
            label: target,
        });
    }
    let nodes = nodes.into_iter().collect();
    GraphInfoDto { edges, nodes }
}

/// Get a serialized representation of the workspace graph.
pub async fn workspace_graph_repr(workspace: &Workspace) -> GraphInfoDto {
    let labeled_graph = workspace.projects.labeled_graph();
    extract_nodes_and_edges_from_graph(&labeled_graph)
}

/// Get a serialized representation of the dependency graph.
pub async fn dep_graph_repr(graph: &DepGraph) -> GraphInfoDto {
    let labeled_graph = graph.labeled_graph();
    extract_nodes_and_edges_from_graph(&labeled_graph)
}

pub fn respond_to_request(
    server: Server,
    tera: &mut Tera,
    graph: &GraphInfoDto,
) -> Result<(), AnyError> {
    info!(
        target: LOG_TARGET,
        r#"Starting server on "{}""#,
        server.server_addr()
    );
    for req in server.incoming_requests() {
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
                // Use the local version of the JS file when in development mode otherwise
                // the CDN url.
                let mut js_url = match cfg!(debug_assertions) {
                    // FIXME: We should create a separate module to store these constants
                    true => "http://localhost:5000".to_string(),
                    false => "https://cdn.com".to_string(),
                };
                js_url.push_str("/assets/index.js");
                let context = RenderContext { graph_data, js_url };
                let info = tera
                    .render_str(INDEX_HTML, &Context::from_serialize(&context)?)
                    .unwrap();
                let mut response = Response::from_data(info);
                response.add_header(
                    Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..]).unwrap(),
                );
                response
            }
        };
        req.respond(response).unwrap_or_default();
    }
    Ok(())
}
