use console::style;
use moon_logger::color;

const STEP_CHAR: &str = "▪";
const PASS_COLORS: [u8; 4] = [57, 63, 69, 75];
const FAIL_COLORS: [u8; 4] = [124, 125, 126, 127];
const MUTED_COLORS: [u8; 4] = [240, 242, 244, 246];
const SETUP_COLORS: [u8; 4] = [198, 205, 212, 219];

pub enum Checkpoint {
    RunFailed,
    RunPassed,
    RunStart,
    Setup,
}

#[inline]
pub fn label_moon() -> String {
    format!(
        "{}{}{}{}",
        style("m").color256(PASS_COLORS[0]).bold(),
        style("o").color256(PASS_COLORS[1]).bold(),
        style("o").color256(PASS_COLORS[2]).bold(),
        style("n").color256(PASS_COLORS[3]).bold(),
    )
}

#[inline]
pub fn label_to_the_moon() -> String {
    vec![
        style("❯").color256(55),
        style("❯❯").color256(56),
        style("❯ t").color256(57),
        style("o t").color256(63),
        style("he ").color256(69),
        style("mo").color256(75),
        style("on").color256(81),
    ]
    .iter()
    .map(|i| i.to_string())
    .collect::<Vec<String>>()
    .join("")
}

#[inline]
pub fn label_checkpoint<T: AsRef<str>>(label: T, checkpoint: Checkpoint) -> String {
    let colors = match checkpoint {
        Checkpoint::RunFailed => FAIL_COLORS,
        Checkpoint::RunPassed => PASS_COLORS,
        Checkpoint::RunStart => MUTED_COLORS,
        Checkpoint::Setup => SETUP_COLORS,
    };

    format!(
        "{}{}{}{} {}",
        color::paint(colors[0], STEP_CHAR),
        color::paint(colors[1], STEP_CHAR),
        color::paint(colors[2], STEP_CHAR),
        color::paint(colors[3], STEP_CHAR),
        style(label.as_ref()).bold()
    )
}

#[inline]
pub fn print_checkpoint<T: AsRef<str>>(label: T, checkpoint: Checkpoint) {
    println!("{}", label_checkpoint(label, checkpoint));
}

#[inline]
pub fn label_header<T: AsRef<str>>(label: T) -> String {
    style(format!(" {} ", label.as_ref().to_uppercase()))
        .bold()
        .reverse()
        .to_string()
}
