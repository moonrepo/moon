use crate::helpers::{replace_style_tokens, safe_exit};
use console::{measure_text_width, style, Attribute, Style, Term};
use core::fmt::Debug;
use moon_logger::color;
use moon_logger::color::Color;
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
    fn format_label<V: AsRef<str>>(&self, kind: Label, message: V) -> String;

    // RENDERERS

    fn render_entry<K: AsRef<str>, V: AsRef<str>>(&self, key: K, value: V) -> TermWriteResult;
    fn render_entry_list<K: AsRef<str>, V: AsRef<[String]>>(
        &self,
        key: K,
        values: V,
    ) -> TermWriteResult;
    fn render_error(&self, error: Box<dyn std::error::Error>) -> !;
    fn render_label<V: AsRef<str>>(&self, kind: Label, message: V) -> TermWriteResult;
    fn render_list<V: AsRef<[String]>>(&self, values: V) -> TermWriteResult;
}

impl ExtendedTerm for Term {
    fn format(&self, value: &impl Debug) -> String {
        format!("{value:?}")
    }

    fn format_label<V: AsRef<str>>(&self, kind: Label, message: V) -> String {
        let mut style = Style::new()
            .attr(Attribute::Bold)
            .color256(Color::Black as u8);

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
            .apply_to(format!(" {} ", message.as_ref()).to_uppercase())
            .to_string()
    }

    fn render_entry<K: AsRef<str>, V: AsRef<str>>(&self, key: K, value: V) -> TermWriteResult {
        let label = color::muted_light(format!("{}:", style(key.as_ref()).bold()));

        self.write_line(&format!("{} {}", label, value.as_ref()))
    }

    fn render_entry_list<K: AsRef<str>, V: AsRef<[String]>>(
        &self,
        key: K,
        values: V,
    ) -> TermWriteResult {
        let label = color::muted_light(format!("{}:", style(key.as_ref()).bold()));

        self.write_line(&label)?;
        self.render_list(values)?;

        Ok(())
    }

    #[track_caller]
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

    fn render_label<V: AsRef<str>>(&self, kind: Label, message: V) -> TermWriteResult {
        self.write_line(&self.format_label(kind, message.as_ref()))?;
        self.write_line("")?;

        Ok(())
    }

    fn render_list<V: AsRef<[String]>>(&self, values: V) -> TermWriteResult {
        for value in values.as_ref() {
            self.write_line(&format!(" {} {}", color::muted("-"), value))?;
        }

        Ok(())
    }
}
