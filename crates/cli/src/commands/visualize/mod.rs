mod dto;
mod resolver;
mod schema;
mod service;

use crate::helpers::AnyError;

use async_graphql::{http::GraphiQLSource, EmptyMutation, EmptySubscription, Schema};
use async_graphql_rocket::{GraphQLRequest, GraphQLResponse};
use moon_logger::info;
use rocket::http::ContentType;
use rocket::response::content::RawHtml;
use rocket::{get, post, routes, State};
use rust_embed::RustEmbed;
use std::borrow::Cow;
use std::ffi::OsStr;
use std::path::PathBuf;

use self::resolver::QueryRoot;

const INDEX_HTML: &str = "index.html";
pub type AppSchema = Schema<QueryRoot, EmptyMutation, EmptySubscription>;

#[derive(RustEmbed)]
#[folder = "../../apps/visualizer/dist"]
struct Assets;

pub async fn visualize() -> Result<(), AnyError> {
    info!("Starting visualizer on {}", "http://127.0.0.1:8000");

    let schema = schema::build_schema().await?;
    #[allow(unused_must_use)]
    let _rocket = rocket::build()
        .manage(schema)
        .mount("/", routes![index, other_files, graphiql, graphql_request])
        .launch()
        .await?;

    Ok(())
}

#[get("/")]
fn index() -> Option<RawHtml<Cow<'static, [u8]>>> {
    let asset = Assets::get(INDEX_HTML)?;
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

#[get("/graphiql")]
fn graphiql() -> RawHtml<String> {
    RawHtml(GraphiQLSource::build().endpoint("/graphql").finish())
}

#[post("/graphql", data = "<request>", format = "application/json")]
async fn graphql_request(schema: &State<AppSchema>, request: GraphQLRequest) -> GraphQLResponse {
    request.execute(schema).await
}
