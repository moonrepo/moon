pub mod server;
pub mod start;
pub mod stop;

use clap::Subcommand;

#[derive(Clone, Debug, Subcommand)]
pub enum DaemonCommands {
    #[command(
        name = "start",
        alias = "startup",
        about = "Start the daemon.",
        long_about = "Start the daemon if it's not running. If already running, will reuse the PID."
    )]
    Start,

    #[command(
        name = "stop",
        alias = "shutdown",
        about = "Stop the daemon.",
        long_about = "Stop the daemon if it's running. Will attempt to shutdown gracefully, otherwise the process will be killed."
    )]
    Stop,

    #[command(name = "server", hide = true)]
    Server,
}
