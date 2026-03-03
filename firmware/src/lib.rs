//! Water Flow ESP32 Firmware
//!
//! Main library entry point that re-exports all modules and provides the orchestrator.

#![allow(unused_imports)]

pub mod config;
pub mod engine;
pub mod mqtt;
pub mod sensor;
pub mod time;
pub mod traits;
pub mod wifi;

#[cfg(target_os = "espidf")]
pub fn run() -> anyhow::Result<()> {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    log::info!("Water flow monitor starting...");

    // Take ownership of all peripherals
    let esp_idf_hal::peripherals::Peripherals { pins, modem, .. } = esp_idf_hal::peripherals::Peripherals::take().map_err(|_| anyhow::anyhow!("Failed to take peripherals"))?;

    // Configure flow sensor interrupt (consumes pins)
    sensor::setup_flow_sensor(pins)?;
    log::info!("Flow sensor interrupt configured");

    // Shared concurrent state
    let wifi_connected = Arc::new(AtomicBool::new(false));
    let mqtt_connected = Arc::new(AtomicBool::new(false));
    let mqtt_client = Arc::new(Mutex::new(None));

    // Spawn WiFi management thread
    let wifi_connected_clone = wifi_connected.clone();
    std::thread::spawn(move || match wifi::setup_wifi(modem) {
        Ok(wifi_adapter) => {
            if let Err(e) = wifi::run_wifi_loop(wifi_adapter, wifi_connected_clone) {
                log::error!("WiFi thread error: {:?}", e);
            }
        }
        Err(e) => log::error!("WiFi setup failed: {:?}", e),
    });

    // Spawn MQTT management thread
    let mqtt_connected_clone = mqtt_connected.clone();
    let mqtt_client_clone = mqtt_client.clone();
    let wifi_connected_mqtt = wifi_connected.clone();
    std::thread::spawn(move || {
        if let Err(e) = mqtt::run_mqtt_manager_thread(
            wifi_connected_mqtt,
            mqtt_connected_clone,
            mqtt_client_clone,
        ) {
            log::error!("MQTT thread error: {:?}", e);
        }
    });

    // Run core flow monitoring loop with generic dependencies
    engine::runner(
        sensor::EspFlowCounter,
        time::EspTimeSource,
        time::EspDelay,
        mqtt_client,
        mqtt_connected,
        config::MQTT_TOPIC,
        0, // QoS::AtMostOnce as i8
    )?;

    Ok(())
}
