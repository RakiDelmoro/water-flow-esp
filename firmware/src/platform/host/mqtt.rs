use crate::platform::traits::{MqttManager, MqttPublisher};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

#[derive(Debug, Clone)]
pub struct CapturedPublish {
    pub topic: String,
    pub retain: bool,
    pub payload: Vec<u8>,
}

impl CapturedPublish {
    fn new(topic: &str, retain: bool, payload: &[u8]) -> Self {
        Self {
            topic: topic.to_owned(),
            retain,
            payload: payload.to_vec(),
        }
    }
}

pub struct HostMqttPublisher {
    pub published: Vec<CapturedPublish>,
    pub fail_next: bool,
}

impl HostMqttPublisher {
    pub fn new() -> Self {
        Self {
            published: Vec::new(),
            fail_next: false,
        }
    }

    pub fn last_published(&self) -> Option<&CapturedPublish> {
        self.published.last()
    }

    pub fn all_payloads(&self) -> Vec<&[u8]> {
        self.published
            .iter()
            .map(|c| c.payload.as_slice())
            .collect()
    }
}

impl MqttPublisher for HostMqttPublisher {
    fn publish(&mut self, topic: &str, retain: bool, payload: &[u8]) -> anyhow::Result<()> {
        match std::mem::replace(&mut self.fail_next, false) {
            true => anyhow::bail!("injected MQTT publish failure"),
            false => {
                self.published
                    .push(CapturedPublish::new(topic, retain, payload));
                Ok(())
            }
        }
    }
}

pub struct HostMqttManager;

impl MqttManager for HostMqttManager {
    type Client = HostMqttPublisher;

    fn run_loop(
        _config: &crate::config::Config,
        wifi_ready: Arc<AtomicBool>,
        mqtt_ready: Arc<AtomicBool>,
        client_slot: Arc<Mutex<Option<Self::Client>>>,
        shutdown: Option<Arc<AtomicBool>>,
    ) -> anyhow::Result<()> {
        loop {
            // Global shutdown check (top of loop)
            if let Some(s) = &shutdown {
                if s.load(Ordering::Relaxed) {
                    mqtt_ready.store(false, Ordering::Relaxed);
                    return Ok(());
                }
            }

            // Wait for WiFi ready
            while !wifi_ready.load(Ordering::Relaxed) {
                if let Some(s) = &shutdown {
                    if s.load(Ordering::Relaxed) {
                        mqtt_ready.store(false, Ordering::Relaxed);
                        return Ok(());
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(1));
            }

            // WiFi ready: simulate MQTT connection
            let publisher = HostMqttPublisher::new();
            *client_slot.lock().unwrap() = Some(publisher);
            mqtt_ready.store(true, Ordering::Relaxed);

            // Stay connected while WiFi remains up
            while wifi_ready.load(Ordering::Relaxed) {
                if let Some(s) = &shutdown {
                    if s.load(Ordering::Relaxed) {
                        mqtt_ready.store(false, Ordering::Relaxed);
                        // Do NOT clear slot; preserve for inspection after join
                        return Ok(());
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(1));
            }

            // WiFi dropped
            mqtt_ready.store(false, Ordering::Relaxed);
            *client_slot.lock().unwrap() = None;
            std::thread::sleep(std::time::Duration::from_millis(10)); // backoff before reconnect
                                                                      // Loop back to wait for WiFi again
        }
    }
}
