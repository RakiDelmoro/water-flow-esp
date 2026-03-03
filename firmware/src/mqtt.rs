use crate::traits::MqttPublisher;
use anyhow::{anyhow, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use log::info;

/// Platform-agnostic MQTT event type
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MqttEvent {
    Connected,
    Disconnected,
    Other,
}

/// Trait for MQTT event source
pub trait MqttConnection: Send {
    fn next(&mut self) -> Result<MqttEvent>;
}

/// Core MQTT management logic (platform-agnostic)
pub fn step_mqtt_manager<P, C>(
    wifi_connected: &Arc<AtomicBool>,
    mqtt_connected: &Arc<AtomicBool>,
    mqtt_client: &Arc<Mutex<Option<Box<dyn MqttPublisher>>>>,
    builder: impl FnOnce() -> Result<(P, C)>,
) -> Result<()>
where
    P: MqttPublisher + 'static,
    C: MqttConnection + 'static,
{
    if !wifi_connected.load(Ordering::Relaxed) {
        if mqtt_connected.load(Ordering::Relaxed) {
            info!("WiFi disconnected - clearing MQTT state");
            mqtt_connected.store(false, Ordering::Relaxed);
            if let Ok(mut guard) = mqtt_client.lock() {
                *guard = None;
            }
        }
        thread::sleep(Duration::from_millis(100));
        return Ok(());
    }

    info!("WiFi ready - initializing MQTT...");
    match builder() {
        Ok((publisher, mut connection)) => {
            info!("MQTT client created, waiting for connection...");

            if let Ok(mut guard) = mqtt_client.lock() {
                *guard = Some(Box::new(publisher));
            }

            let mut connected = false;
            'event: loop {
                match connection.next() {
                    Ok(MqttEvent::Connected) => {
                        if !connected {
                            info!("MQTT connected!");
                            mqtt_connected.store(true, Ordering::Relaxed);
                            connected = true;
                        }
                    }
                    Ok(MqttEvent::Disconnected) => {
                        info!("MQTT Disconnected");
                        break 'event;
                    }
                    Ok(MqttEvent::Other) => {}
                    Err(e) => {
                        info!("MQTT event error: {:?}", e);
                        break 'event;
                    }
                }
            }

            info!("MQTT disconnected - will reconnect when ready");
            mqtt_connected.store(false, Ordering::Relaxed);
            if let Ok(mut guard) = mqtt_client.lock() {
                *guard = None;
            }
            thread::sleep(Duration::from_secs(1));
        }
        Err(e) => {
            info!("Failed to setup MQTT: {:?}, will retry...", e);
            thread::sleep(Duration::from_secs(1));
        }
    }
    Ok(())
}

#[cfg(target_os = "espidf")]
mod impl_esp {
    use super::*;
    use crate::config::{MQTT_PASSWORD, MQTT_URL, MQTT_USERNAME};
    use esp_idf_svc::mqtt::client::{
        EspMqttClient, EspMqttConnection, EventPayload, MqttClientConfiguration, QoS,
    };

    #[derive(Debug)]
    pub struct EspMqttPublisher {
        client: EspMqttClient<'static>,
    }

    impl EspMqttPublisher {
        pub fn new(client: EspMqttClient<'static>) -> Self {
            Self { client }
        }
    }

    impl MqttPublisher for EspMqttPublisher {
        fn publish(&mut self, topic: &str, data: &[u8], qos: i8, retain: bool) -> Result<()> {
            let qos_enum = match qos {
                0 => QoS::AtMostOnce,
                1 => QoS::AtLeastOnce,
                2 => QoS::ExactlyOnce,
                _ => return Err(anyhow!("invalid QoS: {}", qos)),
            };
            self.client.publish(topic, qos_enum, retain, data)?;
            Ok(())
        }
    }

    pub struct EspConnection {
        inner: EspMqttConnection,
    }

    impl MqttConnection for EspConnection {
        fn next(&mut self) -> Result<MqttEvent> {
            match self.inner.next() {
                Ok(ev) => match ev.payload() {
                    EventPayload::Connected(_) => Ok(MqttEvent::Connected),
                    EventPayload::Disconnected => Ok(MqttEvent::Disconnected),
                    _ => Ok(MqttEvent::Other),
                },
                Err(e) => Err(anyhow::anyhow!("MQTT error: {:?}", e)),
            }
        }
    }

    pub fn setup() -> Result<(EspMqttPublisher, EspConnection)> {
        let cfg = MqttClientConfiguration {
            client_id: Some("esp-water-flow"),
            username: Some(MQTT_USERNAME),
            password: Some(MQTT_PASSWORD),
            ..Default::default()
        };
        let (client, conn) = EspMqttClient::new(MQTT_URL, &cfg)?;
        Ok((EspMqttPublisher::new(client), EspConnection { inner: conn }))
    }

    pub fn step_default(
        wifi_connected: &Arc<AtomicBool>,
        mqtt_connected: &Arc<AtomicBool>,
        mqtt_client: &Arc<Mutex<Option<Box<dyn MqttPublisher>>>>,
    ) -> Result<()> {
        step_mqtt_manager(wifi_connected, mqtt_connected, mqtt_client, setup)
    }

    pub fn run_thread(
        wifi_connected: Arc<AtomicBool>,
        mqtt_connected: Arc<AtomicBool>,
        mqtt_client: Arc<Mutex<Option<Box<dyn MqttPublisher>>>>,
    ) -> Result<()> {
        loop {
            step_default(&wifi_connected, &mqtt_connected, &mqtt_client)?;
        }
    }
}

#[cfg(target_os = "espidf")]
pub use impl_esp::{run_thread, step_default, EspMqttPublisher};

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;
    use std::cell::Cell;

    #[derive(Debug)]
    struct MockConn {
        events: Vec<MqttEvent>,
        idx: Cell<usize>,
    }

    impl MockConn {
        fn new(events: Vec<MqttEvent>) -> Self {
            Self {
                events,
                idx: std::cell::Cell::new(0),
            }
        }
    }

    impl MqttConnection for MockConn {
        fn next(&mut self) -> Result<MqttEvent> {
            let idx = self.idx.get();
            if idx < self.events.len() {
                let ev = self.events[idx];
                self.idx.set(idx + 1);
                Ok(ev)
            } else {
                Err(anyhow::anyhow!("no more events"))
            }
        }
    }

    mock! {
        Pub {}
        impl MqttPublisher for Pub {
            fn publish(&mut self, topic: &str, data: &[u8], qos: i8, retain: bool) -> Result<()>;
        }
    }

    #[test]
    fn mqtt_sets_connected_flag_when_connected() {
        use std::cell::Cell;

        let wifi_connected = Arc::new(AtomicBool::new(true));
        let mqtt_connected = Arc::new(AtomicBool::new(false));
        let mqtt_client = Arc::new(Mutex::new(None));

        // Custom connection that verifies the flag is set after Connected event
        struct CheckConn {
            flag: Arc<AtomicBool>,
            stage: Cell<u8>,
        }

        impl MqttConnection for CheckConn {
            fn next(&mut self) -> Result<MqttEvent> {
                match self.stage.get() {
                    0 => {
                        self.stage.set(1);
                        Ok(MqttEvent::Connected)
                    }
                    1 => {
                        // At this point, step_mqtt_manager should have set the flag to true
                        assert!(
                            self.flag.load(Ordering::Relaxed),
                            "mqtt_connected should be true after Connected event"
                        );
                        Ok(MqttEvent::Disconnected)
                    }
                    _ => Err(anyhow::anyhow!("unexpected event request")),
                }
            }
        }

        let conn = CheckConn {
            flag: mqtt_connected.clone(),
            stage: Cell::new(0),
        };

        let builder = || Ok((MockPub::new(), conn));

        step_mqtt_manager(&wifi_connected, &mqtt_connected, &mqtt_client, builder).unwrap();

        // After step returns, Disconnected was processed, so flag should be cleared
        assert!(!mqtt_connected.load(Ordering::Relaxed));
    }

    #[test]
    fn mqtt_clears_connected_flag_when_disconnected() {
        let wifi_connected = Arc::new(AtomicBool::new(true));
        let mqtt_connected = Arc::new(AtomicBool::new(true));
        let mqtt_client = Arc::new(Mutex::new(None));

        // Simulate: immediately disconnected
        let conn = MockConn::new(vec![MqttEvent::Disconnected]);

        let builder = || Ok((MockPub::new(), conn));

        step_mqtt_manager(&wifi_connected, &mqtt_connected, &mqtt_client, builder).unwrap();

        assert!(!mqtt_connected.load(Ordering::Relaxed));
        assert!(mqtt_client.lock().unwrap().is_none());
    }

    #[test]
    fn mqtt_clears_state_when_wifi_down() {
        let wifi_connected = Arc::new(AtomicBool::new(false));
        let mqtt_connected = Arc::new(AtomicBool::new(true));
        let mqtt_client = Arc::new(Mutex::new(None));

        let builder = || Ok((MockPub::new(), MockConn::new(vec![])));

        step_mqtt_manager(&wifi_connected, &mqtt_connected, &mqtt_client, builder).unwrap();

        assert!(!mqtt_connected.load(Ordering::Relaxed));
        assert!(mqtt_client.lock().unwrap().is_none());
    }

    #[test]
    fn mqtt_builder_error_does_not_change_state() {
        let wifi_connected = Arc::new(AtomicBool::new(true));
        let mqtt_connected = Arc::new(AtomicBool::new(false));
        let mqtt_client = Arc::new(Mutex::new(None));

        let builder = || -> Result<(MockPub, MockConn)> { Err(anyhow::anyhow!("fail")) };

        step_mqtt_manager(&wifi_connected, &mqtt_connected, &mqtt_client, builder).unwrap();

        assert!(!mqtt_connected.load(Ordering::Relaxed));
        assert!(mqtt_client.lock().unwrap().is_none());
    }
}
