use crate::console::Console;
use starbase_styles::color::owo::OwoColorize;
use starbase_styles::color::{self, OwoStyle};

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
    RunStarted,
    Setup,
}

impl Console {
    pub fn format_checkpoint<M: AsRef<str>>(
        &self,
        checkpoint: Checkpoint,
        message: M,
        comments: &[String],
    ) -> String {
        let colors = match checkpoint {
            Checkpoint::Announcement => ANNOUNCEMENT_COLORS,
            Checkpoint::RunFailed => FAIL_COLORS,
            Checkpoint::RunPassed => PASS_COLORS,
            Checkpoint::RunStarted => MUTED_COLORS,
            Checkpoint::Setup => SETUP_COLORS,
        };

        format!(
            "{}{}{}{} {} {}",
            color::paint(colors[0], STEP_CHAR),
            color::paint(colors[1], STEP_CHAR),
            color::paint(colors[2], STEP_CHAR),
            color::paint(colors[3], STEP_CHAR),
            OwoStyle::new().style(message.as_ref()).bold(),
            self.format_comments(comments),
        )
    }

    pub fn format_comments(&self, comments: &[String]) -> String {
        if comments.is_empty() {
            return String::new();
        }

        color::muted(format!("({})", comments.join(", ")))
    }

    pub fn format_header<M: AsRef<str>>(&self, message: M) -> String {
        OwoStyle::new()
            .style(message.as_ref())
            .bold()
            .reversed()
            .to_string()
    }

    pub fn print_checkpoint<M: AsRef<str>>(&self, checkpoint: Checkpoint, message: M) {
        self.print_checkpoint_with_comments(checkpoint, message, &[]);
    }

    pub fn print_checkpoint_with_comments<M: AsRef<str>>(
        &self,
        checkpoint: Checkpoint,
        message: M,
        comments: &[String],
    ) {
        if !self.quiet {
            self.write_line(
                self.format_checkpoint(checkpoint, message, comments)
                    .into_bytes(),
            );
        }
    }
}
