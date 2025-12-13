mod lookup;
mod shared;

use starbase::MainResult;
use std::env;
use std::ffi::OsString;

#[tokio::main]
async fn main() -> MainResult {
    shared::run_cli({
        let mut args = env::args_os().collect::<Vec<_>>();
        args.insert(1, OsString::from("exec"));
        args
    })
    .await
}
