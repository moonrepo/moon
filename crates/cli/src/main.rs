mod lookup;
mod shared;
mod stdio;

use starbase::MainResult;
use std::env;

#[tokio::main]
async fn main() -> MainResult {
    shared::run_cli(env::args_os().collect()).await
}
