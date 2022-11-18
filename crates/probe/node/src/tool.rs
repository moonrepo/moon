use probe_core::Tool;
use std::marker::PhantomData;

pub struct NodeLanguage<'tool> {
    pub version: String,

    data: &'tool PhantomData<()>,
}

impl<'tool> Tool<'tool> for NodeLanguage<'tool> {}
