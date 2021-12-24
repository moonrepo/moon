use fern::Dispatch;
use std::io;

pub mod color;

pub struct Logger {
    first_log: bool,

    last_timestamp: u32,
}

impl Logger {
    pub fn new() -> Self {
        Logger {
            first_log: true,
            last_timestamp: 0,
        }
    }

    pub fn init(&self) {
        Dispatch::new()
            .format(|out, message, record| {
                out.finish(format_args!(
                    "{}[{}][{}] {}",
                    color::muted(if self.first_log { "" } else { "" }), // chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                    record.target(),
                    record.level(),
                    message
                ))
            })
            // Pipe errors to stderr
            .chain(
                Dispatch::new()
                    .level(log::LevelFilter::Error)
                    .chain(io::stderr()),
            )
            // All other log types go to stdout
            .chain(
                fern::Dispatch::new()
                    .level(log::LevelFilter::Debug)
                    .chain(io::stdout()),
            )
            .apply()
            .unwrap();
    }
}
