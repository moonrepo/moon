use miette::IntoDiagnostic;
use moon_common::color;
use moon_env_var::GlobalEnvBag;
use moon_process::ProcessRegistry;
use serde::Serialize;
use std::sync::Arc;
use tera::{Context, Tera};
use tiny_http::{Header, Request, Response, Server};
use tokio::task::{JoinHandle, spawn};

const INDEX_HTML: &str = include_str!("html.tera");

#[derive(Debug, Serialize)]
pub struct RenderContext {
    pub page_title: String,
    pub graph_data: String, // JSON
    pub js_url: String,
}

pub async fn setup_server(host: String, port: u16) -> miette::Result<(Arc<Server>, Tera)> {
    let address = format!("{host}:{port}");
    let server = Server::http(address).unwrap();
    let tera = Tera::default();

    Ok((Arc::new(server), tera))
}

pub fn respond_to_request(
    req: Request,
    tera: &mut Tera,
    graph_data: &str,
    page_title: String,
) -> miette::Result<()> {
    let response = match req.url() {
        "/graph-data" => {
            let mut response = Response::from_data(graph_data);
            response.add_header(
                Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap(),
            );
            response
        }
        _ => {
            let context = RenderContext {
                page_title,
                graph_data: graph_data.into(),
                js_url: get_js_url(),
            };

            let info = tera
                .render_str(
                    INDEX_HTML,
                    &Context::from_serialize(context).into_diagnostic()?,
                )
                .into_diagnostic()?;

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

pub async fn run_server(
    title: &str,
    graph_data: String,
    host: String,
    port: u16,
) -> miette::Result<()> {
    let (server, mut tera) = setup_server(host, port).await?;
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
            respond_to_request(req, &mut tera, &graph_data, title.clone())?;
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
