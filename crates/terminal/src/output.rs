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

pub fn label_to_the_moon() -> String {
    vec![
        // style("❯").color256(238),
        // style("❯").color256(242),
        // style("❯").color256(246),
        // style("❯").color256(250),
        // style("❯").color256(255),
        // style("❯").color256(229),
        // style(" "),
        // style("🆃").color256(55),
        // style("🅾").color256(56),
        // style(" "),
        // style("🆃").color256(57),
        // style("🅷").color256(57),
        // style("🅴").color256(63),
        // style(" "),
        // style("🅼").color256(63),
        // style("🅾").color256(69),
        // style("🅾").color256(75),
        // style("🅽").color256(81),
        //
        // style("❯").color256(56),
        // style("❯").color256(57),
        // style("❯").color256(63),
        // style("❯").color256(69),
        // style("❯").color256(75),
        // style("❯").color256(81),
        // style(" 🌑"),
        //
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

    // ∙∙∙∙∙·▫▫ᵒᴼᵒ▫∙∙▫ᵒᴼᵒ▫∙∙▫ᵒᴼᵒ☼)===>
    // format!(
    //     // "{}{}{}🚀🌑",
    //     "{}{}{}{}{}{} 🆃🅾 🆃🅷🅴 🅼🅾🅾🅽",
    //     // "{} 🆃 🅾  {} 🆃 🅷 🅴  {} 🅼 🅾 🅾 🅽",
    //     // "{}{}{} 🅃🄾 🅃🄷🄴 🄼🄾🄾🄽",
    //     style("❯").color256(238),
    //     style("❯").color256(242),
    //     style("❯").color256(246),
    //     style("❯").color256(250),
    //     style("❯").color256(255),
    //     style("❯").color256(229),
    //     // style("··").color256(248),
    //     // style("∙∙∙").color256(244),
    //     // style("•••").color256(249)
    // )
}

pub fn label_checkpoint<T: AsRef<str>>(label: T, checkpoint: Checkpoint) -> String {
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
        style(label.as_ref()).bold()
    )
}
