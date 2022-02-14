use console::style;
use moon_logger::color;

const STEP_CHAR: &str = "▪";

pub fn label_moon() -> String {
    format!(
        "{}{}{}{}",
        style(color::paint(57, "m")).bold(),
        style(color::paint(63, "o")).bold(),
        style(color::paint(69, "o")).bold(),
        style(color::paint(75, "n")).bold(),
    )
}

pub fn label_run_target(target: &str) -> String {
    format!(
        "{}{}{}{} {}",
        color::paint(57, STEP_CHAR),
        color::paint(63, STEP_CHAR),
        color::paint(69, STEP_CHAR),
        color::paint(75, STEP_CHAR),
        style(target).bold()
    )
}

pub fn label_run_target_failed(target: &str) -> String {
    format!(
        "{}{}{}{} {}",
        color::paint(124, STEP_CHAR),
        color::paint(125, STEP_CHAR),
        color::paint(126, STEP_CHAR),
        color::paint(127, STEP_CHAR),
        style(target).bold()
    )
}
