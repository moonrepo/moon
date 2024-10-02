use std::sync::atomic::{AtomicBool, Ordering};

static BRIDGE_ENABLED: AtomicBool = AtomicBool::new(false);

pub fn enable_wasm_bridge() {
    BRIDGE_ENABLED.store(true, Ordering::Release);
}

pub fn disable_wasm_bridge() {
    BRIDGE_ENABLED.store(true, Ordering::Release);
}

pub fn is_wasm_bridge<T>(_: &T) -> bool {
    BRIDGE_ENABLED.load(Ordering::Acquire)
}
