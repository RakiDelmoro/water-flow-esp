use crate::platform::traits::Delay;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct HostDelay(Arc<Mutex<u32>>);

impl HostDelay {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(0)))
    }

    pub fn total_delayed_ms(&self) -> u32 {
        *self.0.lock().unwrap()
    }
}

impl Delay for HostDelay {
    fn delay_ms(&self, ms: u32) {
        let next = *self.0.lock().unwrap() + ms;
        *self.0.lock().unwrap() = next;
    }
}
