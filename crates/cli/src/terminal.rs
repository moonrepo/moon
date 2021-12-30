use crate::helpers::safe_exit;
use crate::output::replace_style_tokens;
use console::{Attribute, Style, StyledObject, Term};
use moon_logger::color::Color;
use std::io;

pub enum Label {
    Failure,
    Success,
}

pub struct Terminal {
    term: Term,
}

impl Terminal {
    pub fn stdout() -> Self {
        Terminal {
            term: Term::stdout(),
        }
    }

    pub fn stderr() -> Self {
        Terminal {
            term: Term::stdout(),
        }
    }

    pub fn format_label(&self, kind: Label, message: &str) -> StyledObject<String> {
        let mut style = Style::new().attr(Attribute::Bold);

        match kind {
            Label::Failure => {
                style = style
                    .color256(Color::White as u8)
                    .on_color256(Color::Red as u8);
            }
            Label::Success => {
                style = style
                    .color256(Color::Black as u8)
                    .on_color256(Color::Green as u8);
            }
        }

        style.apply_to(format!(" {} ", message).to_uppercase())
    }

    pub fn render_error(&self, error: Box<dyn std::error::Error>) -> ! {
        let contents = format!(
            "{} {}",
            self.format_label(Label::Failure, "Error"),
            &replace_style_tokens(&error.to_string())
        );

        self.block(&contents, 1).unwrap();

        safe_exit(1);
    }
}

// LAYOUT

pub type LayoutResult = io::Result<()>;

impl Terminal {
    pub fn block(&self, contents: &str, padding: u8) -> LayoutResult {
        let y = String::from("\n").repeat(padding as usize);
        let x = String::from(" ").repeat(padding as usize);

        self.term.write_str(&y)?;
        self.term.write_line(&format!("{}{}{}", x, contents, x))?;
        self.term.write_str(&y)?;

        Ok(())
    }
}
