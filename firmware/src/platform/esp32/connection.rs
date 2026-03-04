use crate::platform::traits::ConnectionGuard;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub struct Esp32ConnectionGuard {
    wifi_ready: Arc<AtomicBool>,
    mqtt_ready: Arc<AtomicBool>,
}

impl Esp32ConnectionGuard {
    pub fn new(wifi_ready: Arc<AtomicBool>, mqtt_ready: Arc<AtomicBool>) -> Self {
        Self {
            wifi_ready,
            mqtt_ready,
        }
    }
}

impl ConnectionGuard for Esp32ConnectionGuard {
    fn is_ready(&self) -> bool {
        [&self.wifi_ready, &self.mqtt_ready]
            .iter()
            .all(|f| f.load(Ordering::Relaxed))
    }
}
