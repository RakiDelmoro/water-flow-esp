use crate::platform::traits::Clock;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct HostClock(Arc<Mutex<u64>>);

impl HostClock {
    pub fn new(initial_ms: u64) -> Self {
        Self(Arc::new(Mutex::new(initial_ms)))
    }

    pub fn advance(&self, ms: u64) {
        let next = *self.0.lock().unwrap() + ms;
        *self.0.lock().unwrap() = next;
    }
}

impl Clock for HostClock {
    fn time_now_in_millis(&self) -> u64 {
        *self.0.lock().unwrap()
    }
}
