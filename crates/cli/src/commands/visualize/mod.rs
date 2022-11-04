use crate::helpers::AnyError;

use moon_logger::{info, trace};
use portpicker::is_free;
use rocket::http::ContentType;
use rocket::response::content::RawHtml;
use rocket::{get, routes};
use rust_embed::RustEmbed;
use std::borrow::Cow;
use std::ffi::OsStr;
use std::{env, net::SocketAddr, path::PathBuf};

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

    #[allow(unused_must_use)]
    let _rocket = rocket::build()
        .mount("/", routes![index, other_files])
        .launch()
        .await?;

    Ok(())
}

#[get("/")]
fn index() -> Option<RawHtml<Cow<'static, [u8]>>> {
    let asset = Assets::get("index.html")?;
    Some(RawHtml(asset.data))
}

#[get("/<file..>")]
fn other_files(file: PathBuf) -> Option<(ContentType, Cow<'static, [u8]>)> {
    let filename = file.display().to_string();
    let asset = Assets::get(&filename)?;
    let content_type = file
        .extension()
        .and_then(OsStr::to_str)
        .and_then(ContentType::from_extension)
        .unwrap_or(ContentType::Bytes);
    Some((content_type, asset.data))
}
