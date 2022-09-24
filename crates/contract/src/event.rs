pub enum EventFlow {
    Break,
    Continue,
    Return(String),
}

#[macro_export]
macro_rules! handle_flow {
    ($result:expr) => {
        match $result? {
            EventFlow::Break => return Ok(EventFlow::Break),
            EventFlow::Return(value) => return Ok(EventFlow::Return(value)),
            EventFlow::Continue => {}
        };
    };
}
