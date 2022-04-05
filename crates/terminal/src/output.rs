use console::style;
use moon_logger::color;

const STEP_CHAR: &str = "▪";

pub fn label_moon() -> String {
    format!(
        "{}{}{}{}",
        style("M").color256(57).bold(),
        style("O").color256(63).bold(),
        style("O").color256(69).bold(),
        style("N").color256(75).bold(),
    )
}

pub fn label_run_target(target_id: &str) -> String {
    format!(
        "{}{}{}{} {}",
        color::paint(57, STEP_CHAR),
        color::paint(63, STEP_CHAR),
        color::paint(69, STEP_CHAR),
        color::paint(75, STEP_CHAR),
        style(target_id).bold()
    )
}

pub fn label_run_target_failed(target_id: &str) -> String {
    format!(
        "{}{}{}{} {}",
        color::paint(124, STEP_CHAR),
        color::paint(125, STEP_CHAR),
        color::paint(126, STEP_CHAR),
        color::paint(127, STEP_CHAR),
        style(target_id).bold()
    )
}

pub fn bold(value: &str) -> String {
    format!("{}", style(value).bold())
}
