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
        wifi_ready: Arc<AtomicBool>,
        mqtt_ready: Arc<AtomicBool>,
        client_slot: Arc<Mutex<Option<Self::Client>>>,
    ) -> anyhow::Result<()> {
        let session = wifi_ready
            .load(Ordering::Relaxed)
            .then(HostMqttPublisher::new);
        let is_ready = session.is_some();
        *client_slot.lock().unwrap() = session;
        mqtt_ready.store(is_ready, Ordering::Relaxed);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publisher_fail_next_recovers_on_next_call() {
        let mut p = HostMqttPublisher {
            published: vec![],
            fail_next: true,
        };
        assert!(p.publish("t", false, b"x").is_err());
        assert!(p.publish("t", false, b"x").is_ok());
        assert_eq!(p.published.len(), 1);
    }

    #[test]
    fn manager_populates_slot_when_wifi_ready() {
        let wifi = Arc::new(AtomicBool::new(true));
        let mqtt = Arc::new(AtomicBool::new(false));
        let slot: Arc<Mutex<Option<HostMqttPublisher>>> = Arc::new(Mutex::new(None));
        HostMqttManager::run_loop(wifi, mqtt.clone(), slot.clone()).unwrap();
        assert!(mqtt.load(Ordering::Relaxed));
        assert!(slot.lock().unwrap().is_some());
    }

    #[test]
    fn manager_leaves_slot_empty_when_wifi_not_ready() {
        let wifi = Arc::new(AtomicBool::new(false));
        let mqtt = Arc::new(AtomicBool::new(false));
        let slot: Arc<Mutex<Option<HostMqttPublisher>>> = Arc::new(Mutex::new(None));
        HostMqttManager::run_loop(wifi, mqtt.clone(), slot).unwrap();
        assert!(!mqtt.load(Ordering::Relaxed));
    }
}
