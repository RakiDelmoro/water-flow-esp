use std::sync::{Arc, Mutex};
use crate::platform::traits::Clock;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advances_deterministically() {
        let c = HostClock::new(0);
        c.advance(1_000);
        c.advance(500);
        assert_eq!(c.time_now_in_millis(), 1_500);
    }

    #[test]
    fn elapsed_reflects_advance() {
        let c = HostClock::new(1_000);
        c.advance(300);
        assert_eq!(c.elapsed_ms(1_000), 300);
    }
}
