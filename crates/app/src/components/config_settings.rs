use iocraft::prelude::*;
use moon_console::ui::{Stack, Style, StyledText, View};
use schematic::{Schema, SchemaType};

#[derive(Default, Props)]
pub struct ConfigSettingsProps<'a> {
    pub schema: Option<&'a Schema>,
}

#[component]
pub fn ConfigSettings<'a>(props: &ConfigSettingsProps<'a>) -> impl Into<AnyElement<'a>> {
    let Some(SchemaType::Struct(config)) = props.schema.as_ref().map(|schema| &schema.ty) else {
        return element!(View).into_any();
    };

    element! {
        Stack(gap: 1) {
            #(config.fields.iter().map(|(field, setting)| {
                let mut flags = vec![];

                if setting.deprecated.is_some() {
                    flags.push("deprecated");
                }

                if !setting.optional {
                    flags.push("required");
                }

                element! {
                    Stack {
                        View {
                            StyledText(
                                content: format!(
                                    "<property>{}</property><muted>:</muted> {} {}",
                                    field,
                                    setting.schema,
                                    if flags.is_empty() {
                                        "".to_string()
                                    } else {
                                        format!(
                                            "<muted>({})</muted>",
                                            flags.join(", ")
                                        )
                                    }
                                )
                            )
                        }
                        #(setting.comment.as_ref().map(|comment| {
                            element! {
                                View {
                                    StyledText(
                                        content: comment,
                                        style: Style::MutedLight
                                    )
                                }
                            }
                        }))
                    }
                }.into_any()
            }))
        }
    }
    .into_any()
}
