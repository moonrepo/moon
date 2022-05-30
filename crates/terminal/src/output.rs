use console::style;
use moon_logger::color;

const STEP_CHAR: &str = "▪";

const PASS_COLORS: [u8; 4] = [57, 63, 69, 75];
const FAIL_COLORS: [u8; 4] = [124, 125, 126, 127];
const MUTED_COLORS: [u8; 4] = [240, 242, 244, 246];

pub enum Checkpoint {
    Fail,
    Pass,
    Start,
}

pub fn label_moon() -> String {
    format!(
        "{}{}{}{}",
        style("m").color256(PASS_COLORS[0]).bold(),
        style("o").color256(PASS_COLORS[1]).bold(),
        style("o").color256(PASS_COLORS[2]).bold(),
        style("n").color256(PASS_COLORS[3]).bold(),
    )
}

pub fn label_checkpoint(label: &str, checkpoint: Checkpoint) -> String {
    let colors = match checkpoint {
        Checkpoint::Fail => FAIL_COLORS,
        Checkpoint::Pass => PASS_COLORS,
        Checkpoint::Start => MUTED_COLORS,
    };

    format!(
        "{}{}{}{} {}",
        color::paint(colors[0], STEP_CHAR),
        color::paint(colors[1], STEP_CHAR),
        color::paint(colors[2], STEP_CHAR),
        color::paint(colors[3], STEP_CHAR),
        style(label).bold()
    )
}

pub fn bold(value: &str) -> String {
    format!("{}", style(value).bold())
}
