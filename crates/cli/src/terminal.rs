use crate::helpers::safe_exit;
use console::{Attribute, Style, StyledObject, Term};

use moon_logger::color::Color;

pub enum Label {
    Failure,
    Success,
}

pub struct Terminal {}

impl Terminal {
    pub fn label<D>(kind: Label, message: D) -> StyledObject<D> {
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

        style.apply_to(message)
    }

    pub fn render_error(error: Box<dyn std::error::Error>) -> ! {
        let term = Term::stderr();

        term.write_line(&format!(
            "{} {}",
            Terminal::label(Label::Failure, "Error"),
            error,
        ))
        .unwrap();

        term.flush().unwrap();

        safe_exit(1);
    }
}
