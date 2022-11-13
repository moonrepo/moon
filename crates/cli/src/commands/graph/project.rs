use crate::{commands::graph::utils::workspace_info, helpers::load_workspace};
use serde::Serialize;
use std::env;
use tera::{Context, Tera};
use tiny_http::{Header, Response, Server};

const INDEX_HTML: &str = include_str!("index.html.tera");

#[derive(Debug, Serialize)]
struct RenderContext {
    workspace_info: String,
    js_url: String,
}

pub async fn project_graph(project_id: &Option<String>) -> Result<(), Box<dyn std::error::Error>> {
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

    let mut tera = Tera::default();
    let workspace = load_workspace().await?;
    workspace.projects.load_all()?;
    let workspace_info = workspace_info(&workspace).await;

    for req in server.incoming_requests() {
        let response = match req.url() {
            "/graph-data" => {
                let mut response = Response::from_data(serde_json::to_string(&workspace_info)?);
                response.add_header(
                    Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap(),
                );
                response
            }
            _ => {
                let workspace_info = serde_json::to_string(&workspace_info)?;
                // FIXME: We should create a separate module to store these constants
                let mut js_url = match cfg!(debug_assertions) {
                    true => "http://localhost:5000".to_string(),
                    false => "https://cdn.com".to_string(),
                };
                js_url.push_str("/assets/index.js");
                let context = RenderContext {
                    workspace_info,
                    js_url,
                };
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

    if let Some(id) = project_id {
        project_build.load(id)?;
    } else {
        project_build.load_all()?;
    }

    let project_graph = project_build.build();

    println!("{}", project_graph.to_dot());

    Ok(())
}
