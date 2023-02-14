use crate::is_test_env;
// use chrono::Duration;
// use chrono_humanize::HumanTime;
use std::time::{Duration, SystemTime};

pub use chrono;
pub use humantime::{format_duration, parse_duration};

pub fn now_timestamp() -> chrono::NaiveDateTime {
    chrono::Utc::now().naive_utc()
}

pub fn now_millis() -> u128 {
    to_millis(SystemTime::now())
}

pub fn to_millis(time: SystemTime) -> u128 {
    match time.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => d.as_millis(),
        Err(_) => 0,
    }
}

pub fn is_stale(timestamp: u128, duration: Duration) -> bool {
    timestamp == 0 || now_millis() >= timestamp + duration.as_millis()
}

pub fn elapsed(duration: Duration) -> String {
    if is_test_env() {
        return String::from("100ms"); // Snapshots
    }

    let secs = duration.as_secs();
    let nanos = duration.subsec_nanos();

    if secs == 0 && nanos == 0 {
        return String::from("0s");
    }

    let years = secs / 31_557_600;
    let year_days = secs % 31_557_600;
    let months = year_days / 2_630_016;
    let month_days = year_days % 2_630_016;
    let days = month_days / 86400;
    let day_secs = month_days % 86400;
    let hours = day_secs / 3600;
    let minutes = day_secs % 3600 / 60;
    let seconds = day_secs % 60;
    let millis = nanos / 1_000_000;
    let mut parts = vec![];

    if years > 0 {
        parts.push(format!("{years}y"));
    }

    if months > 0 {
        parts.push(format!("{months}mo"));
    }

    if days > 0 {
        parts.push(format!("{days}d"));
    }

    if hours > 0 {
        parts.push(format!("{hours}h"));
    }

    if minutes > 0 {
        parts.push(format!("{minutes}m"));
    }

    if seconds > 0 {
        parts.push(format!("{seconds}s"));
    }

    if millis > 0 {
        parts.push(format!("{millis}ms"));
    }

    if parts.is_empty() {
        parts.push(String::from("0s"))
    }

    parts.join(" ")
}

// pub fn relative(duration: Duration) -> String {
//     format!("{}", HumanTime::from(duration))
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_durations() {
        assert_eq!(parse_duration("7 days").unwrap(), Duration::new(604800, 0));
    }

    #[test]
    #[should_panic(expected = "UnknownUnit")]
    fn handles_invalid_durations() {
        parse_duration("7 unknown").unwrap();
    }
}
