use crate::context::RunState;
use starbase_events::Event;

macro_rules! impl_event {
    ($name:ident, $impl:tt) => {
        impl_event!($name, (), $impl);
    };

    ($name:ident, $data:ty, $impl:tt) => {
        #[derive(Debug)]
        pub struct $name $impl

        impl Event for $name {
            type Data = $data;
        }
    };
}

impl_event!(JobStateChangeEvent, {
    pub job: String,
    pub state: RunState,
    pub prev_state: RunState,
});

impl_event!(JobProgressEvent, {
    pub job: String,
    pub elapsed: u32,
});
