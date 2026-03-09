//! MQTT Manager - Event-Driven with Fast-Fail
//!
//! Self-powered: try once, fail fast, no retry delays.

use crate::main_config::{MQTT_PASSWORD, MQTT_URL, MQTT_USERNAME};
use crate::AppEvent;
use esp_idf_svc::mqtt::client::{EspMqttClient, EventPayload, MqttClientConfiguration};
use log::{info, warn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc::Sender, Arc, Mutex};

/// Setup MQTT client
pub fn setup_mqtt() -> anyhow::Result<(EspMqttClient<'static>, esp_idf_svc::mqtt::client::EspMqttConnection)> {
    let config = MqttClientConfiguration {
        client_id: Some("esp-water-flow"),
        username: Some(MQTT_USERNAME),
        password: Some(MQTT_PASSWORD),
        ..Default::default()
    };

    let (client, conn) = EspMqttClient::new(MQTT_URL, &config)?;
    info!(target: "mqtt", "Client created");

    Ok((client, conn))
}

/// Run MQTT loop - event-driven, fast-fail
pub fn run_mqtt_loop(is_mqtt_connected: Arc<AtomicBool>, mqtt_client: Arc<Mutex<Option<EspMqttClient<'static>>>>, event_tx: Arc<Mutex<Sender<AppEvent>>>) -> anyhow::Result<()> {
    let (client, mut connection) = match setup_mqtt() {
        Ok(c) => c,
        Err(e) => {
            warn!(target: "mqtt", "Setup failed: {:?}", e);
            return Ok(()); // Fast-fail, no retry
        }
    };

    // Store client
    if let Ok(mut guard) = mqtt_client.lock() {
        *guard = Some(client);
    }

    let mut confirmed = false;

    loop {
        match connection.next() {
            Ok(event) => match event.payload() {
                EventPayload::Connected(_) => {
                    if !confirmed {
                        info!(target: "mqtt", "Connected");
                        is_mqtt_connected.store(true, Ordering::Relaxed);
                        confirmed = true;

                        if let Ok(tx) = event_tx.lock() {
                            let _ = tx.send(AppEvent::MqttState(true));
                        }
                    }
                }
                EventPayload::Disconnected => {
                    warn!(target: "mqtt", "Disconnected");
                    is_mqtt_connected.store(false, Ordering::Relaxed);

                    if let Ok(mut guard) = mqtt_client.lock() {
                        *guard = None;
                    }

                    if let Ok(tx) = event_tx.lock() {
                        let _ = tx.send(AppEvent::MqttState(false));
                    }

                    // Fast-fail: exit
                    return Ok(());
                }
                _ => {}
            },
            Err(e) => {
                warn!(target: "mqtt", "Error: {:?}", e);
                is_mqtt_connected.store(false, Ordering::Relaxed);

                if let Ok(mut guard) = mqtt_client.lock() {
                    *guard = None;
                }

                if let Ok(tx) = event_tx.lock() {
                    let _ = tx.send(AppEvent::MqttState(false));
                }

                // Fast-fail: exit
                return Ok(());
            }
        }
    }
}
