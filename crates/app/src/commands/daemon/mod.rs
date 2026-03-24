pub mod logs;
pub mod restart;
pub mod server;
pub mod start;
pub mod status;
pub mod stop;

use clap::Subcommand;

#[derive(Clone, Debug, Subcommand)]
pub enum DaemonCommands {
    #[command(
        name = "logs",
        about = "Tail the daemon's logs.",
        long_about = "Tail the daemon's logs. If the daemon is not running, this will fail."
    )]
    Logs,

    #[command(
        name = "restart",
        about = "Retart the daemon.",
        long_about = "Restart the daemon by attempting to stop the currently running process, and then start a new process."
    )]
    Restart,

    #[command(
        name = "start",
        alias = "startup",
        about = "Start the daemon.",
        long_about = "Start the daemon if it's not running. If already running, will reuse the PID."
    )]
    Start,

    #[command(
        name = "status",
        alias = "stats",
        about = "View status of the daemon.",
        long_about = "View status of the daemon if it's running."
    )]
    Status,

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
