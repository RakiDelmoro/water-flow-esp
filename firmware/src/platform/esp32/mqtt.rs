use crate::platform::traits::{MqttManager, MqttPublisher};
use esp_idf_svc::mqtt::client::{EspMqttClient, MqttClientConfiguration, QoS};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

pub struct Esp32MqttPublisher {
    client: EspMqttClient<'static>,
}

impl MqttPublisher for Esp32MqttPublisher {
    fn publish(&mut self, topic: &str, retain: bool, payload: &[u8]) -> anyhow::Result<()> {
        self.client
            .publish(topic, QoS::AtLeastOnce, retain, payload)
            .map(|_| ())
            .map_err(anyhow::Error::from)
    }
}

pub struct Esp32MqttManager;

impl MqttManager for Esp32MqttManager {
    type Client = Esp32MqttPublisher;

    fn run_loop(
        config: &crate::config::Config,
        wifi_ready: Arc<AtomicBool>,
        mqtt_ready: Arc<AtomicBool>,
        client_slot: Arc<Mutex<Option<Self::Client>>>,
    ) -> anyhow::Result<()> {
        std::iter::repeat(())
            .try_for_each(|_| session_loop(config, &wifi_ready, &mqtt_ready, &client_slot))
    }
}

fn session_loop(
    config: &crate::config::Config,
    wifi_ready: &Arc<AtomicBool>,
    mqtt_ready: &Arc<AtomicBool>,
    client_slot: &Arc<Mutex<Option<Esp32MqttPublisher>>>,
) -> anyhow::Result<()> {
    std::iter::repeat(())
        .take_while(|_| !wifi_ready.load(Ordering::Relaxed))
        .for_each(|_| std::thread::sleep(std::time::Duration::from_millis(500)));

    let cfg = MqttClientConfiguration {
        client_id: Some(&config.mqtt_client_id),
        username: config.mqtt_username.as_deref(),
        password: config.mqtt_password.as_deref(),
        ..Default::default()
    };

    EspMqttClient::new(&config.mqtt_broker_url, &cfg)
        .map_err(anyhow::Error::from)
        .map(|(client, connection)| {
            *client_slot.lock().unwrap() = Some(Esp32MqttPublisher { client });
            mqtt_ready.store(true, Ordering::Relaxed);
            log::info!("MQTT connected.");
            connection
        })
        .map(|mut conn| {
            std::iter::from_fn(|| conn.next().ok())
                .for_each(|event| log::debug!("MQTT event: {:?}", event.payload()));
        })
        .unwrap_or_else(|e| log::error!("MQTT error: {e}, retrying..."));

    mqtt_ready.store(false, Ordering::Relaxed);
    *client_slot.lock().unwrap() = None;
    std::thread::sleep(std::time::Duration::from_secs(3));
    Ok(())
}
