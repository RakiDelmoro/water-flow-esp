use crate::platform::traits::ConnectionGuard;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

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

    pub fn new_with_arcs(wifi_ready: Arc<AtomicBool>, mqtt_ready: Arc<AtomicBool>) -> Self {
        Self {
            wifi_ready,
            mqtt_ready,
        }
    }

    pub fn set_wifi(&self, ready: bool) {
        self.wifi_ready.store(ready, Ordering::Relaxed)
    }
    pub fn set_mqtt(&self, ready: bool) {
        self.mqtt_ready.store(ready, Ordering::Relaxed)
    }
}

impl ConnectionGuard for HostConnectionGuard {
    fn is_ready(&self) -> bool {
        [&self.wifi_ready, &self.mqtt_ready]
            .iter()
            .all(|f| f.load(Ordering::Relaxed))
    }
}
