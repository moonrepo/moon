use crate::helpers::safe_exit;
use crate::output::replace_style_tokens;
use console::{measure_text_width, Attribute, Style, Term};
use moon_logger::color::Color;
use std::io;

pub enum Label {
    Failure,
    // Success,
}

pub type TermLayoutResult = io::Result<()>;

// Extend `Term` with our own methods

pub trait ExtendedTerm {
    fn format_label(&self, kind: Label, message: &str) -> String;
    fn render_error(&self, error: Box<dyn std::error::Error>) -> !;

    // LAYOUT

    fn block(&self, contents: &str, padding: u8) -> TermLayoutResult;
}

impl ExtendedTerm for Term {
    fn format_label(&self, kind: Label, message: &str) -> String {
        let mut style = Style::new().attr(Attribute::Bold);

        match kind {
            Label::Failure => {
                style = style
                    .color256(Color::White as u8)
                    .on_color256(Color::Red as u8);
            } // Label::Success => {
              //     style = style
              //         .color256(Color::Black as u8)
              //         .on_color256(Color::Green as u8);
              // }
        }

        style
            .apply_to(format!(" {} ", message).to_uppercase())
            .to_string()
    }

    fn render_error(&self, error: Box<dyn std::error::Error>) -> ! {
        let label = self.format_label(Label::Failure, "Error");
        let label_width = measure_text_width(&label);
        let message = replace_style_tokens(error.to_string().trim());
        let message_width = measure_text_width(&message);
        let available_space = self.size().1 as usize - label_width - 3; // padding
        let contents;

        if message.contains('\n') || message_width > available_space {
            contents = format!("{}\n\n{}", label, &message);
        } else {
            contents = format!("{} {}", label, &message);
        }

        self.block(&contents, 1).unwrap();
        self.flush().unwrap();

        safe_exit(1);
    }

    // LAYOUT

    fn block(&self, contents: &str, padding: u8) -> TermLayoutResult {
        if padding == 0 {
            self.write_line(contents)?;

            return Ok(());
        }

        let y = String::from("\n").repeat(padding as usize);
        let x = String::from(" ").repeat(padding as usize);

        self.write_str(&y)?;

        for line in contents.split('\n') {
            self.write_line(&format!("{}{}{}", x, line, x))?;
        }

        self.write_str(&y)?;

        Ok(())
    }
}
