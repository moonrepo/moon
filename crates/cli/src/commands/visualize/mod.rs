mod init;

use crate::helpers::AnyError;

use moon_logger::info;
use rocket::http::ContentType;
use rocket::response::content::RawHtml;
use rocket::{get, routes};
use rust_embed::RustEmbed;
use std::borrow::Cow;
use std::ffi::OsStr;
use std::path::PathBuf;

#[derive(RustEmbed)]
#[folder = "../../apps/visualizer/dist"]
struct Assets;

pub async fn visualize() -> Result<(), AnyError> {
    info!("Starting visualizer on {}", "http://127.0.0.1:8000");
    let workspace = init::init().await?;

    #[allow(unused_must_use)]
    let _rocket = rocket::build()
        .manage(workspace)
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
