use crate::console::Console;
use starbase_styles::color::owo::{OwoColorize, XtermColors};
use starbase_styles::color::{self, Color, OwoStyle};

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

fn bold(message: &str) -> String {
    OwoStyle::new().style(message).bold().to_string()
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
            bold(message.as_ref()),
            self.format_comments(comments),
        )
    }

    pub fn format_comments(&self, comments: &[String]) -> String {
        if comments.is_empty() {
            return String::new();
        }

        color::muted(format!("({})", comments.join(", ")))
    }

    pub fn format_entry_key<K: AsRef<str>>(&self, key: K) -> String {
        color::muted_light(format!("{}:", bold(key.as_ref())))
    }

    pub fn print_checkpoint<M: AsRef<str>>(
        &self,
        checkpoint: Checkpoint,
        message: M,
    ) -> miette::Result<()> {
        self.print_checkpoint_with_comments(checkpoint, message, &[])
    }

    pub fn print_checkpoint_with_comments<M: AsRef<str>>(
        &self,
        checkpoint: Checkpoint,
        message: M,
        comments: &[String],
    ) -> miette::Result<()> {
        if !self.quiet {
            self.write_line(self.format_checkpoint(checkpoint, message, comments))?;
        }

        Ok(())
    }

    pub fn print_line(&self) -> miette::Result<()> {
        self.write("\n".to_owned().into_bytes())
    }

    pub fn print_entry<K: AsRef<str>, V: AsRef<str>>(
        &self,
        key: K,
        value: V,
    ) -> miette::Result<()> {
        self.write_line(format!("{} {}", self.format_entry_key(key), value.as_ref()))
    }

    pub fn print_entry_bool<K: AsRef<str>>(&self, key: K, value: bool) -> miette::Result<()> {
        self.print_entry(key, if value { "Yes" } else { "No" })
    }

    pub fn print_entry_list<K: AsRef<str>, V: AsRef<[String]>>(
        &self,
        key: K,
        values: V,
    ) -> miette::Result<()> {
        self.write_line(self.format_entry_key(key).into_bytes())?;
        self.print_list(values)?;

        Ok(())
    }

    pub fn print_entry_header<M: AsRef<str>>(&self, message: M) -> miette::Result<()> {
        self.print_line()?;
        self.write_line(
            OwoStyle::new()
                .style(format!(" {} ", message.as_ref().to_uppercase()))
                .bold()
                .reversed()
                .to_string(),
        )?;
        self.print_line()?;

        Ok(())
    }

    pub fn print_header<M: AsRef<str>>(&self, message: M) -> miette::Result<()> {
        self.print_line()?;
        self.write_line(
            OwoStyle::new()
                .style(format!(" {} ", message.as_ref().to_uppercase()))
                .bold()
                .color(XtermColors::from(Color::Black as u8))
                .on_color(XtermColors::from(Color::Purple as u8))
                .to_string(),
        )?;
        self.print_line()?;

        Ok(())
    }

    pub fn print_list<V: AsRef<[String]>>(&self, values: V) -> miette::Result<()> {
        let mut values = values.as_ref().to_owned();
        values.sort();

        for value in values {
            self.write_line(format!(" {} {}", color::muted("-"), value))?;
        }

        Ok(())
    }
}
