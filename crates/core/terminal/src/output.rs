use console::style;
use starbase_styles::color;

const STEP_CHAR: &str = "▪";
const PASS_COLORS: [u8; 4] = [57, 63, 69, 75];
const FAIL_COLORS: [u8; 4] = [124, 125, 126, 127];
const MUTED_COLORS: [u8; 4] = [240, 242, 244, 246];
const SETUP_COLORS: [u8; 4] = [198, 205, 212, 219];
const ANNOUNCEMENT_COLORS: [u8; 4] = [35, 42, 49, 86];

pub enum Checkpoint {
    Announcement,
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
pub fn get_checkpoint_prefix(checkpoint: Checkpoint) -> String {
    let colors = match checkpoint {
        Checkpoint::Announcement => ANNOUNCEMENT_COLORS,
        Checkpoint::RunFailed => FAIL_COLORS,
        Checkpoint::RunPassed => PASS_COLORS,
        Checkpoint::RunStart => MUTED_COLORS,
        Checkpoint::Setup => SETUP_COLORS,
    };

    format!(
        "{}{}{}{}",
        color::paint(colors[0], STEP_CHAR),
        color::paint(colors[1], STEP_CHAR),
        color::paint(colors[2], STEP_CHAR),
        color::paint(colors[3], STEP_CHAR),
    )
}

#[inline]
pub fn label_checkpoint<T: AsRef<str>>(label: T, checkpoint: Checkpoint) -> String {
    format!(
        "{} {}",
        get_checkpoint_prefix(checkpoint),
        style(label.as_ref()).bold()
    )
}

#[inline]
pub fn print_checkpoint<T: AsRef<str>>(label: T, checkpoint: Checkpoint) {
    println!("{}", label_checkpoint(label, checkpoint));
}
