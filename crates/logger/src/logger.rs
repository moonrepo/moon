use crate::color;
use chrono::prelude::*;
use chrono::Local;
use fern::Dispatch;
use log::LevelFilter;
use std::env;
use std::io;

static mut FIRST_LOG: bool = true;
static mut LAST_HOUR: u32 = 0;

pub struct Logger {}

impl Logger {
    pub fn init(level: LevelFilter) {
        if level == LevelFilter::Off {
            return;
        }

        Dispatch::new()
            .filter(|metadata| metadata.target().starts_with("moon"))
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
            .chain(Dispatch::new().level(level).chain(io::stderr()))
            .apply()
            .unwrap();
    }
}
