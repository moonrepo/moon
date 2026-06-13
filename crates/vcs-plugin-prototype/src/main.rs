//! Executable conformance and benchmark harness for source-control providers.

mod benchmark;
mod conformance;
mod plugin;

use miette::{IntoDiagnostic, miette};

#[tokio::main]
async fn main() -> miette::Result<()> {
    let workspace_root = std::env::current_dir().into_diagnostic()?;
    let args = std::env::args().skip(1).collect::<Vec<_>>();

    match args.as_slice() {
        [] => conformance::run(&workspace_root).await,
        [arg] if arg == "--conformance" => conformance::run(&workspace_root).await,
        [arg] if arg == "--benchmark" => benchmark::run(&workspace_root, false).await,
        [arg] if arg == "--benchmark-check" => benchmark::run(&workspace_root, true).await,
        [
            arg,
            master_binary,
            current_binary,
            master_fixture,
            current_fixture,
        ] if arg == "--benchmark-git-comparison" => benchmark::run_git_comparison(
            std::path::Path::new(master_binary),
            std::path::Path::new(current_binary),
            std::path::Path::new(master_fixture),
            std::path::Path::new(current_fixture),
        ),
        _ => Err(miette!(
            "unknown provider harness arguments: {}",
            args.join(" ")
        )),
    }
}
