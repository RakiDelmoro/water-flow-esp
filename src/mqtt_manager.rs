use crate::main_config::{MQTT_PASSWORD, MQTT_URL, MQTT_USERNAME};
use esp_idf_svc::mqtt::client::{EspMqttClient, EventPayload, MqttClientConfiguration};
use log::info;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub fn setup_mqtt() -> anyhow::Result<(EspMqttClient<'static>, esp_idf_svc::mqtt::client::EspMqttConnection)> {
    let mqtt_config = MqttClientConfiguration {
        client_id: Some("esp-water-flow"),
        username: Some(MQTT_USERNAME),
        password: Some(MQTT_PASSWORD),
        keep_alive_interval: Some(std::time::Duration::from_secs(60)),
        network_timeout: std::time::Duration::from_secs(5),
        reconnect_timeout: None, // Disable auto-reconnect to prevent blocking
        ..Default::default()
    };
    let (mqtt_client, mqtt_event_loop) = EspMqttClient::new(MQTT_URL, &mqtt_config)?;

    anyhow::Ok((mqtt_client, mqtt_event_loop))
}

pub fn run_mqtt_loop(wifi_connected: Arc<AtomicBool>, mqtt_connected: Arc<AtomicBool>, mqtt_client: Arc<Mutex<Option<EspMqttClient<'static>>>>) -> anyhow::Result<()> {
    let mut reconnect_attempts: u32 = 0;
    const MAX_BACKOFF_SECS: u64 = 30;
    const CONNECTION_TIMEOUT_SECS: u64 = 10;

    loop {
        // Wait for WiFi to be connected before attempting MQTT
        if !wifi_connected.load(Ordering::Relaxed) {
            // Clear MQTT state if WiFi is down
            if mqtt_connected.load(Ordering::Relaxed) {
                info!("WiFi disconnected - clearing MQTT state");
                mqtt_connected.store(false, Ordering::Relaxed);
                // Don't lock mutex to clear client - will be overwritten later
                reconnect_attempts = 0; // Reset backoff on WiFi disconnect
            }
            thread::sleep(Duration::from_millis(100));
            continue;
        }

        // WiFi is connected - give it a moment to fully stabilize (DHCP/IP ready)
        if reconnect_attempts == 0 {
            thread::sleep(Duration::from_secs(1));
        }

        // WiFi is connected - try to establish MQTT connection
        info!("WiFi ready - initializing MQTT...");
        match setup_mqtt() {
            Ok((client, mut connection)) => {
                info!(
                    "MQTT client created, waiting for connection (timeout {}s)...",
                    CONNECTION_TIMEOUT_SECS
                );

                let start_time = Instant::now();
                let mut connection_confirmed = false;

                while start_time.elapsed() < Duration::from_secs(CONNECTION_TIMEOUT_SECS) {
                    match connection.next() {
                        Ok(event) => {
                            match event.payload() {
                                EventPayload::Connected(_) => {
                                    info!("MQTT connection confirmed!");
                                    connection_confirmed = true;
                                    break;
                                }
                                EventPayload::Disconnected => {
                                    info!("MQTT connection failed during handshake");
                                    break;
                                }
                                _ => {
                                    // Continue waiting for connected event
                                    continue;
                                }
                            }
                        }
                        Err(e) => {
                            info!("MQTT connection error: {:?}", e);
                            break;
                        }
                    }
                }

                if !connection_confirmed {
                    // Failed to connect within timeout - clean up and retry
                    info!(
                        "MQTT connection timeout after {} seconds",
                        CONNECTION_TIMEOUT_SECS
                    );
                    mqtt_connected.store(false, Ordering::Relaxed);
                    // Drop client explicitly (it goes out of scope)
                    reconnect_attempts += 1;
                    let backoff_secs =
                        std::cmp::min(2_u64.pow(reconnect_attempts - 1), MAX_BACKOFF_SECS);
                    info!(
                        "Waiting {} seconds before next MQTT reconnect attempt...",
                        backoff_secs
                    );
                    thread::sleep(Duration::from_secs(backoff_secs));
                    continue;
                }

                {
                    let mut guard = mqtt_client
                        .lock()
                        .map_err(|_| anyhow::anyhow!("Mutex poisoned"))?;
                    *guard = Some(client);
                }
                mqtt_connected.store(true, Ordering::Relaxed);
                reconnect_attempts = 0; // Reset counter on successful connection
                info!("MQTT connected and client ready for publishing!");

                loop {
                    match connection.next() {
                        Ok(event) => {
                            match event.payload() {
                                EventPayload::Disconnected => {
                                    info!("MQTT Disconnected by broker");
                                    break; // Exit inner loop to reconnect
                                }
                                _ => {
                                    // Ignore other events (SendingComplete, Received, etc.)
                                }
                            }
                        }
                        Err(e) => {
                            info!("MQTT event error: {:?}", e);
                            break; // Exit inner loop to reconnect
                        }
                    }
                }

                // Clean up before reconnecting
                info!("MQTT disconnected - will reconnect when ready");
                mqtt_connected.store(false, Ordering::Relaxed);
                // Note: we don't clear the client from mutex here to avoid blocking.
                // The old client will be overwritten on next successful connection.

                reconnect_attempts += 1;
                let backoff_secs =
                    std::cmp::min(2_u64.pow(reconnect_attempts - 1), MAX_BACKOFF_SECS);
                info!(
                    "Waiting {} seconds before next MQTT reconnect attempt...",
                    backoff_secs
                );
                thread::sleep(Duration::from_secs(backoff_secs));
            }
            Err(e) => {
                info!("Failed to setup MQTT: {:?}, will retry...", e);
                reconnect_attempts += 1;
                let backoff_secs =
                    std::cmp::min(2_u64.pow(reconnect_attempts - 1), MAX_BACKOFF_SECS);
                thread::sleep(Duration::from_secs(backoff_secs));
            }
        }
    }
}
