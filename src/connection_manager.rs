use crate::main_config::{
    GATEWAY, MQTT_PASSWORD, MQTT_URL, MQTT_USERNAME, NETMASK, STATIC_IP, WIFI_PASSWORD, WIFI_SSID,
};
use anyhow::Result;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::modem::Modem;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::ipv4::{
    ClientConfiguration as IpClientConfiguration, ClientSettings as IpClientSettings,
    Configuration as IpConfiguration, Mask, Subnet,
};
use esp_idf_svc::mqtt::client::{EspMqttClient, EventPayload, MqttClientConfiguration};
use esp_idf_svc::netif::{EspNetif, NetifConfiguration, NetifStack};
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{AuthMethod, ClientConfiguration, Configuration, EspWifi};
use heapless::String;
use log::info;
use std::net::Ipv4Addr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Setup WiFi with static IP or DHCP configuration
pub fn setup_wifi(modem: Modem) -> Result<EspWifi<'static>> {
    let ssid_as_heap_string: String<32> = String::try_from(WIFI_SSID).expect("SSID too long");
    let password_as_heap_string: String<64> = String::try_from(WIFI_PASSWORD).expect("Password too long");

    let sysloop = EspSystemEventLoop::take().expect("Failed to take event loop");
    let nvs = EspDefaultNvsPartition::take().expect("Failed to take NVS");

    let static_ip = Ipv4Addr::new(STATIC_IP[0], STATIC_IP[1], STATIC_IP[2], STATIC_IP[3]);
    let gateway = Ipv4Addr::new(GATEWAY[0], GATEWAY[1], GATEWAY[2], GATEWAY[3]);
    let netmask = Mask(NETMASK);

    let netif_config = NetifConfiguration {
        ip_configuration: Some(IpConfiguration::Client(IpClientConfiguration::Fixed(
            IpClientSettings {
                ip: static_ip,
                subnet: Subnet {
                    gateway,
                    mask: netmask,
                },
                dns: Some(gateway),
                secondary_dns: None,
            },
        ))),
        ..NetifConfiguration::wifi_default_client()
    };

    let sta_netif = EspNetif::new_with_conf(&netif_config)?;
    let ap_netif = EspNetif::new(NetifStack::Ap)?;

    let mut wifi = EspWifi::wrap_all(
        esp_idf_svc::wifi::WifiDriver::new(modem, sysloop.clone(), Some(nvs))?,
        sta_netif,
        ap_netif,
    )?;

    let wifi_config = ClientConfiguration {
        ssid: ssid_as_heap_string,
        password: password_as_heap_string,
        auth_method: AuthMethod::WPA2Personal,
        ..Default::default()
    };

    wifi.set_configuration(&Configuration::Client(wifi_config))
        .expect("Failed to set WiFi configurations");
    wifi.start().expect("Failed to start WiFi");
    wifi.connect().expect("Failed to initiate WiFi connect");

    Ok(wifi)
}

/// Setup MQTT client
pub fn setup_mqtt() -> Result<(
    EspMqttClient<'static>,
    esp_idf_svc::mqtt::client::EspMqttConnection,
)> {
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

    Ok((mqtt_client, mqtt_event_loop))
}

/// Combined connection manager: handles WiFi followed by MQTT, with reconnection logic
pub fn run_connection_loop(
    modem: Modem,
    wifi_connected: Arc<AtomicBool>,
    mqtt_connected: Arc<AtomicBool>,
    mqtt_client: Arc<Mutex<Option<EspMqttClient<'static>>>>,
) -> Result<()> {
    let mut wifi = setup_wifi(modem)?;
    const RECONNECT_DELAY_SECS: u64 = 2;
    const CONNECTION_TIMEOUT_SECS: u64 = 5; // Reduced from 10s to fail faster

    loop {
        // --- WiFi STATE CHECK ---
        let is_wifi_ready = wifi.is_connected()? && wifi.is_up()?;

        if !is_wifi_ready {
            // WiFi DOWN
            if wifi_connected.swap(false, Ordering::Relaxed) {
                info!("WiFi disconnected!");
            }
            if mqtt_connected.swap(false, Ordering::Relaxed) {
                info!("WiFi disconnected - clearing MQTT state");
            }
            // Clear shared MQTT client
            let mut guard = mqtt_client
                .lock()
                .map_err(|_| anyhow::anyhow!("Mutex poisoned"))?;
            *guard = None;

            // WiFi reconnect with fixed delay
            match wifi.connect() {
                Ok(_) => {
                    info!("WiFi reconnection initiated");
                }
                Err(e) => {
                    if e.to_string().contains("ESP_ERR_WIFI_CONN") {
                        // Already connecting, suppress duplicate logs
                    } else {
                        info!("WiFi reconnection error: {:?}", e);
                    }
                }
            }
            let backoff_secs = RECONNECT_DELAY_SECS;
            info!(
                "Waiting {} seconds before next WiFi reconnect attempt...",
                backoff_secs
            );
            FreeRtos::delay_ms((backoff_secs * 1000) as u32);
            continue;
        }

        // --- WiFi UP ---
        if !wifi_connected.load(Ordering::Relaxed) {
            info!("WiFi connected!");
            // Log the configured static IP for diagnostics
            info!(
                "ESP32 static IP configuration: {}.{}.{}.{}",
                STATIC_IP[0], STATIC_IP[1], STATIC_IP[2], STATIC_IP[3]
            );
            wifi_connected.store(true, Ordering::Relaxed);
        }

        // Network stabilization delay - give TCP/IP stack time to be fully ready
        // For water-powered devices, this wait is critical for reliable MQTT connection
        thread::sleep(Duration::from_secs(5));

        // --- MQTT MANAGEMENT ---
        if !mqtt_connected.load(Ordering::Relaxed) {
            info!("WiFi ready - initializing MQTT...");
            info!("MQTT broker URL: {} (user: '{}')", MQTT_URL, MQTT_USERNAME);
            match setup_mqtt() {
                Ok((client, mut connection)) => {
                    info!(
                        "MQTT client created, waiting for connection (timeout {}s)...",
                        CONNECTION_TIMEOUT_SECS
                    );

                    // Wait for Connected event with timeout
                    let start_time = Instant::now();
                    let mut connection_confirmed = false;
                    while start_time.elapsed() < Duration::from_secs(CONNECTION_TIMEOUT_SECS) {
                        match connection.next() {
                            Ok(event) => match event.payload() {
                                EventPayload::Connected(_) => {
                                    info!("MQTT connection confirmed!");
                                    connection_confirmed = true;
                                    break;
                                }
                                EventPayload::Disconnected => {
                                    info!("MQTT connection failed during handshake");
                                    break;
                                }
                                _ => continue,
                            },
                            Err(e) => {
                                info!("MQTT connection error: {:?}", e);
                                break;
                            }
                        }
                    }

                    if !connection_confirmed {
                        info!(
                            "MQTT connection timeout after {} seconds",
                            CONNECTION_TIMEOUT_SECS
                        );
                        mqtt_connected.store(false, Ordering::Relaxed);
                        let backoff_secs = RECONNECT_DELAY_SECS;
                        info!(
                            "Waiting {} seconds before next MQTT reconnect attempt...",
                            backoff_secs
                        );
                        thread::sleep(Duration::from_secs(backoff_secs));
                        continue;
                    }

                    // Store client in shared state
                    {
                        let mut guard = mqtt_client
                            .lock()
                            .map_err(|_| anyhow::anyhow!("Mutex poisoned"))?;
                        *guard = Some(client);
                    }
                    mqtt_connected.store(true, Ordering::Relaxed);
                    info!("MQTT connected and client ready for publishing!");

                    // --- MQTT MONITORING LOOP (blocks until disconnect) ---
                    loop {
                        match connection.next() {
                            Ok(event) => {
                                if let EventPayload::Disconnected = event.payload() {
                                    info!("MQTT Disconnected by broker");
                                    break; // Exit to reconnect
                                }
                                // Ignore other events
                            }
                            Err(e) => {
                                info!("MQTT event error: {:?}", e);
                                break; // Exit to reconnect
                            }
                        }
                    }

                    // Cleanup after disconnect
                    info!("MQTT disconnected - will reconnect when ready");
                    mqtt_connected.store(false, Ordering::Relaxed);
                    let mut guard = mqtt_client
                        .lock()
                        .map_err(|_| anyhow::anyhow!("Mutex poisoned"))?;
                    *guard = None;

                    let backoff_secs = RECONNECT_DELAY_SECS;
                    info!(
                        "Waiting {} seconds before next MQTT reconnect attempt...",
                        backoff_secs
                    );
                    thread::sleep(Duration::from_secs(backoff_secs));
                    // Continue to outer loop → WiFi state check → may reconnect MQTT
                }
                Err(e) => {
                    info!("Failed to setup MQTT: {:?}, will retry...", e);
                    let backoff_secs = RECONNECT_DELAY_SECS;
                    thread::sleep(Duration::from_secs(backoff_secs));
                }
            }
        } else {
            // MQTT is already connected - this should not happen as we'd be in inner monitoring loop
            // But if we somehow get here, just yield and check WiFi again
            thread::sleep(Duration::from_millis(100));
        }
    }
}
