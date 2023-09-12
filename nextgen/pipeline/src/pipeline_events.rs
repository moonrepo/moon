use crate::job::*;
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
    pub state: JobState,
    pub prev_state: JobState,
});

impl_event!(JobProgressEvent, {
    pub job: String,
    pub elapsed: u32,
});

impl_event!(JobFinishedEvent, {
    pub job: String,
    // pub result: JobResult,
});
