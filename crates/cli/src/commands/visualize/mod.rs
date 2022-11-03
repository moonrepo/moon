use crate::helpers::AnyError;
use axum::{
    body::{boxed, Full},
    handler::Handler,
    http::{header, StatusCode, Uri},
    response::Response,
    Router, Server,
};
use moon_logger::{info, trace};
use portpicker::is_free;
use rust_embed::RustEmbed;
use std::{env, net::SocketAddr};

static INDEX_HTML: &str = "index.html";

#[derive(RustEmbed)]
#[folder = "../../apps/visualizer/dist"]
struct Assets;

pub async fn visualize() -> Result<(), AnyError> {
    trace!("Trying to get $PORT from environment variables");
    let mut port = env::var("PORT")
        .map(|p| p.parse::<u16>().expect("Expected $PORT to be a number"))
        .ok();
    if port.is_none() {
        trace!("No environment variable $PORT found, trying to find a random free port");
        for possible_port in 8000..9000 {
            trace!("Checking if {} is free", possible_port);
            if is_free(possible_port) {
                port = Some(possible_port);
                break;
            } else {
                trace!("Port {} is not free, trying next port", possible_port);
            }
        }
    }
    let address = ([0, 0, 0, 0], port.unwrap());
    let addr = SocketAddr::from(address);
    info!("Starting visualizer on {}", addr);
    let app = Router::new().fallback(static_handler.into_service());
    Server::bind(&addr).serve(app.into_make_service()).await?;
    Ok(())
}

async fn static_handler(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    if path.is_empty() || path == INDEX_HTML {
        return index_html().await;
    }
    match Assets::get(path) {
        Some(content) => {
            let body = boxed(Full::from(content.data));
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            Response::builder()
                .header(header::CONTENT_TYPE, mime.as_ref())
                .body(body)
                .unwrap()
        }
        None => {
            if path.contains('.') {
                return not_found().await;
            }
            index_html().await
        }
    }
}

async fn index_html() -> Response {
    match Assets::get(INDEX_HTML) {
        Some(content) => {
            let body = boxed(Full::from(content.data));
            Response::builder()
                .header(header::CONTENT_TYPE, "text/html")
                .body(body)
                .unwrap()
        }
        None => not_found().await,
    }
}

async fn not_found() -> Response {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(boxed(Full::from("404")))
        .unwrap()
}
