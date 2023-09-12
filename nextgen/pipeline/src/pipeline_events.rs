use crate::job::*;
use starbase_events::Event;

macro_rules! impl_event {
    ($name:ident, $impl:tt) => {
        impl_event!($name, (), $impl);
    };

    ($name:ident, $data:ty, $impl:tt) => {
        pub struct $name $impl

        impl Event for $name {
            type Data = $data;
        }
    };
}

impl_event!(JobStateChangeEvent, {
    pub job: String,
    pub state: JobState,
});

impl_event!(JobFinishedEvent, {
    pub job: String,
    pub result: JobResult,
});
