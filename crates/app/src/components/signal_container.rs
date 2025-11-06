use iocraft::prelude::*;
use moon_console::ui::Container;
use moon_process::ProcessRegistry;

#[derive(Default, Props)]
pub struct SignalContainerProps<'a> {
    pub children: Vec<AnyElement<'a>>,
}

#[component]
pub fn SignalContainer<'a>(
    props: &mut SignalContainerProps<'a>,
    mut hooks: Hooks,
) -> impl Into<AnyElement<'a>> {
    hooks.use_future(async move {
        let mut listener = ProcessRegistry::instance().receive_signal();

        if let Ok(signal) = listener.recv().await {
            std::process::exit(128 + signal.get_code());
        }
    });

    element! {
        Container {
            #(&mut props.children)
        }
    }
}
