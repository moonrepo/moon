use crate::helpers::{replace_style_tokens, safe_exit};
use console::{measure_text_width, style, Attribute, Style, Term};
use core::fmt::Debug;
use moon_logger::color;
use moon_logger::color::Color;
use std::env;
use std::io;

pub enum Label {
    Default,
    Brand,
    Failure,
    // Success,
}

pub type TermWriteResult = io::Result<()>;

// Extend `Term` with our own methods

pub trait ExtendedTerm {
    fn format(&self, value: &impl Debug) -> String;
    fn format_label(&self, kind: Label, message: &str) -> String;

    // RENDERERS

    fn render_entry(&self, key: &str, value: &str) -> TermWriteResult;
    fn render_entry_list(&self, key: &str, values: &[String]) -> TermWriteResult;
    fn render_error(&self, error: Box<dyn std::error::Error>) -> !;
    fn render_label(&self, kind: Label, message: &str) -> TermWriteResult;
    fn render_list(&self, values: &[String]) -> TermWriteResult;
}

impl ExtendedTerm for Term {
    fn format(&self, value: &impl Debug) -> String {
        format!("{:?}", value)
    }

    fn format_label(&self, kind: Label, message: &str) -> String {
        let mut style = Style::new()
            .attr(Attribute::Bold)
            .color256(Color::Black as u8);

        // Dont show styles in tests unless we force it
        if env::var("MOON_TEST").is_ok() {
            style = style.force_styling(true);
        }

        match kind {
            Label::Brand => {
                style = style.on_color256(Color::Purple as u8);
            }
            Label::Default => {
                style = style.on_color256(Color::White as u8);
            }
            Label::Failure => {
                style = style
                    .color256(Color::White as u8)
                    .on_color256(Color::Red as u8);
            } // Label::Success => {
              //     style = style.on_color256(Color::Green as u8);
              // }
        }

        style
            .apply_to(format!(" {} ", message).to_uppercase())
            .to_string()
    }

    fn render_entry(&self, key: &str, value: &str) -> TermWriteResult {
        let label = color::muted_light(&format!("{}:", style(key).bold()));

        self.write_line(&format!("{} {}", label, value))
    }

    fn render_entry_list(&self, key: &str, values: &[String]) -> TermWriteResult {
        let label = color::muted_light(&format!("{}:", style(key).bold()));

        self.write_line(&label)?;
        self.render_list(values)?;

        Ok(())
    }

    fn render_error(&self, error: Box<dyn std::error::Error>) -> ! {
        let label = self.format_label(Label::Failure, "Error");
        let label_width = measure_text_width(&label);
        let message = replace_style_tokens(error.to_string().trim());
        let message_width = measure_text_width(&message);
        let available_space = self.size().1 as usize - label_width - 3; // padding

        let contents = if message.contains('\n') || message_width > available_space {
            format!("{}\n\n{}", label, &message)
        } else {
            format!("{} {}", label, &message)
        };

        self.write_line("").unwrap();
        self.write_line(&contents).unwrap();
        self.write_line("").unwrap();
        self.flush().unwrap();

        safe_exit(1);
    }

    fn render_label(&self, kind: Label, message: &str) -> TermWriteResult {
        self.write_line(&self.format_label(kind, message))?;
        self.write_line("")?;

        Ok(())
    }

    fn render_list(&self, values: &[String]) -> TermWriteResult {
        for value in values {
            self.write_line(&format!(" {} {}", color::muted("-"), value))?;
        }

        Ok(())
    }
}
