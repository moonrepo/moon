use crate::color;
use chrono::prelude::*;
use chrono::Local;
use console::strip_ansi_codes;
use fern::log_file;
use fern::Dispatch;
use log::LevelFilter;
use std::env;
use std::io;
use std::path::PathBuf;

static mut FIRST_LOG: bool = true;
static mut LAST_HOUR: u32 = 0;

pub struct Logger {}

impl Logger {
    pub fn init(level: LevelFilter, output: Option<PathBuf>) {
        if level == LevelFilter::Off {
            return;
        }

        let base_logger = Dispatch::new()
            .filter(|metadata| metadata.target().starts_with("moon"))
            .into_shared();

        let colored_logger = Dispatch::new()
            .chain(base_logger.clone())
            .format(|out, message, record| {
                let mut date_format = "%Y-%m-%d %H:%M:%S";
                let current_timestamp = Local::now();

                // Shorten the timestamp when within the same hour
                unsafe {
                    if !FIRST_LOG && current_timestamp.hour() == LAST_HOUR {
                        date_format = "%H:%M:%S";
                    }

                    if FIRST_LOG {
                        FIRST_LOG = false;
                    }

                    if current_timestamp.hour() != LAST_HOUR {
                        LAST_HOUR = current_timestamp.hour();
                    }
                }

                let formatted_timestamp = if env::var("MOON_TEST").is_ok() {
                    String::from("YYYY-MM-DD") // Snapshots
                } else {
                    current_timestamp.format(date_format).to_string()
                };

                let prefix = format!(
                    "{}{} {}{}",
                    color::muted("["),
                    color::log_level(record.level()),
                    color::muted(&formatted_timestamp),
                    color::muted("]"),
                );

                out.finish(format_args!(
                    "{} {} {}",
                    prefix,
                    color::log_target(record.target()),
                    message
                ));
            })
            .chain(io::stderr());

        if let Some(output) = output {
            let file_logger = Dispatch::new()
                .chain(base_logger.clone())
                .format(|out, message, record| {
                    let formatted_timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
                    let prefix = format!("[{} {}]", record.level(), formatted_timestamp);
                    let formatted_message = format!("{} {} {}", prefix, record.target(), message);
                    let message_without_colors = strip_ansi_codes(&formatted_message);

                    out.finish(format_args!("{}", message_without_colors))
                })
                .chain(log_file(output).expect("yikes"));

            Dispatch::new()
                .level(level)
                .chain(file_logger)
                .chain(colored_logger)
                .apply()
                .expect("could not create logger");
        } else {
            Dispatch::new()
                .level(level)
                .chain(colored_logger)
                .apply()
                .expect("could not create logger");
        }
    }
}
