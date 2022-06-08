use crate::is_test_env;
use chrono::Duration;
use chrono_humanize::HumanTime;
use std::time::Duration as StdDuration;

pub use chrono;

pub fn elapsed(duration: StdDuration) -> String {
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
        parts.push(format!("{}y", years));
    }

    if months > 0 {
        parts.push(format!("{}mo", months));
    }

    if days > 0 {
        parts.push(format!("{}d", days));
    }

    if hours > 0 {
        parts.push(format!("{}h", hours));
    }

    if minutes > 0 {
        parts.push(format!("{}m", minutes));
    }

    if seconds > 0 {
        parts.push(format!("{}s", seconds));
    }

    if millis > 0 {
        parts.push(format!("{}ms", millis));
    }

    if parts.is_empty() {
        parts.push(String::from("0s"))
    }

    parts.join(" ")
}

pub fn relative(duration: Duration) -> String {
    format!("{}", HumanTime::from(duration))
}
