use convert_case::{Case, Casing};
use tera::{Kwargs, State};

// STRINGS

pub fn camel_case(value: &str, _: Kwargs, _: &State) -> String {
    value.to_case(Case::Camel)
}

pub fn kebab_case(value: &str, _: Kwargs, _: &State) -> String {
    value.to_case(Case::Kebab)
}

pub fn snake_case(value: &str, _: Kwargs, _: &State) -> String {
    value.to_case(Case::Snake)
}

pub fn pascal_case(value: &str, _: Kwargs, _: &State) -> String {
    value.to_case(Case::Pascal)
}

pub fn title_case(value: &str, _: Kwargs, _: &State) -> String {
    value.to_case(Case::Title)
}
