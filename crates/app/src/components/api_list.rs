use iocraft::prelude::*;
use moon_console::ui::{List, ListItem, Style, StyledText};

#[derive(Default, Props)]
pub struct ApiListProps {
    pub apis: Vec<(String, bool, bool)>,
}

#[component]
pub fn ApiList<'a>(props: &ApiListProps) -> impl Into<AnyElement<'a>> {
    element! {
        List {
            #(props.apis.iter().map(|(api, implemented, required)| {
                element! {
                    ListItem(
                        bullet: if *implemented {
                            "🟢"
                        } else {
                            "⚫️"
                        }.to_owned()
                    ) {
                        StyledText(
                            content: if *required {
                                format!("{api} <muted>(required)</muted>")
                            } else {
                                api.to_string()
                            },
                            style: Style::MutedLight
                        )
                    }
                }
            }))
        }
    }
}
