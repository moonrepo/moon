use crate::helpers::AnyError;
use axum::{response::Html, routing::get, Router, Server};
use moon_logger::{info, trace};
use portpicker::is_free;
use std::net::SocketAddr;

pub async fn visualize() -> Result<(), AnyError> {
    trace!("Trying to get $PORT from environment variables");
    let mut port = std::env::var("PORT")
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
    let app = Router::new().route("/", get(root));
    Server::bind(&addr).serve(app.into_make_service()).await?;
    Ok(())
}

async fn root() -> Html<&'static str> {
    Html(include_str!(
        "../../../../../apps/graph-visualizer/dist/index.html"
    ))
}
