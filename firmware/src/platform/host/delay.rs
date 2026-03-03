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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accumulates_without_real_sleep() {
        let d = HostDelay::new();
        d.delay_ms(100);
        d.delay_ms(200);
        assert_eq!(d.total_delayed_ms(), 300);
    }
}
