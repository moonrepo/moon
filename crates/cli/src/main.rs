mod lookup;
mod shared;

use starbase::MainResult;
use std::env;

#[tokio::main]
async fn main() -> MainResult {
    shared::run_cli(env::args_os().collect()).await
}
