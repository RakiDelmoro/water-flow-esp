use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use crate::platform::traits::ConnectionGuard;

pub struct HostConnectionGuard {
    wifi_ready: Arc<AtomicBool>,
    mqtt_ready: Arc<AtomicBool>,
}

impl HostConnectionGuard {
    pub fn new(ready: bool) -> Self {
        Self {
            wifi_ready: Arc::new(AtomicBool::new(ready)),
            mqtt_ready: Arc::new(AtomicBool::new(ready)),
        }
    }

    pub fn set_wifi(&self, ready: bool) { self.wifi_ready.store(ready, Ordering::Relaxed) }
    pub fn set_mqtt(&self, ready: bool) { self.mqtt_ready.store(ready, Ordering::Relaxed) }
}

impl ConnectionGuard for HostConnectionGuard {
    fn is_ready(&self) -> bool {
        [&self.wifi_ready, &self.mqtt_ready]
            .iter()
            .all(|f| f.load(Ordering::Relaxed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requires_both_flags() {
        let g = HostConnectionGuard::new(true);
        assert!(g.is_ready());
        g.set_wifi(false);
        assert!(!g.is_ready());
        g.set_wifi(true);
        g.set_mqtt(false);
        assert!(!g.is_ready());
    }
}
