//! Entry point for ESP32 firmware.
//!
//! Wires together all concrete platform components and starts the monitoring loop.
//!
//! Build with required environment variables:
//! - WIFI_SSID, WIFI_PASS
//! - MQTT_BROKER_URL, MQTT_CLIENT_ID
//! - MQTT_TOPIC (optional, defaults to "water/flow")
//! - DEVICE_ID (optional, defaults to "esp32-flow")

#![allow(unused_imports)]

// For non-ESP32 targets, provide a dummy main that informs the user.
#[cfg(not(target_os = "espidf"))]
fn main() {
    eprintln!("This firmware binary only runs on ESP32 with the espidf OS.");
    eprintln!("Use `cargo test` for host testing of the library.");
    std::process::exit(1);
}

// For ESP32 targets, the real main function and its imports.
#[cfg(target_os = "espidf")]
use std::sync::{atomic::AtomicBool, Arc, Mutex};

#[cfg(target_os = "espidf")]
use anyhow::Result;

#[cfg(target_os = "espidf")]
use esp_idf_hal::{gpio::Pins, modem::Modem};

#[cfg(target_os = "espidf")]
use esp_idf_svc::log::Logger;

#[cfg(target_os = "espidf")]
use log::info;

#[cfg(target_os = "espidf")]
use firmware::platform::traits::*;

#[cfg(target_os = "espidf")]
use firmware::{config::Config, platform::esp32::*, run};

#[cfg(target_os = "espidf")]
fn main() -> Result<()> {
    // Load configuration from environment
    let config = Config::from_env()?;

    // Initialize logger
    Logger::init_default()?;

    info!("Starting water flow monitor...");

    // Take ownership of hardware peripherals
    let modem = Modem::take()?;
    let pins = Pins::new();
    let flow_pin = take_pin(pins, config.flow_sensor_pin)?;

    // Shared state for WiFi/MQTT readiness and MQTT client
    let wifi_ready = Arc::new(AtomicBool::new(false));
    let mqtt_ready = Arc::new(AtomicBool::new(false));
    let client_slot = Arc::new(Mutex::new(None));

    // Spawn WiFi manager thread
    let wifi_ready_clone = Arc::clone(&wifi_ready);
    let wifi_manager = Esp32WifiManager::setup(modem, &config.wifi_ssid, &config.wifi_pass)?;
    std::thread::spawn(move || {
        if let Err(e) = wifi_manager.run_loop(wifi_ready_clone) {
            log::error!("WiFi task failed: {e}");
        }
    });

    // Spawn MQTT manager thread
    let wifi_ready_clone = Arc::clone(&wifi_ready);
    let mqtt_ready_clone = Arc::clone(&mqtt_ready);
    let client_slot_clone = Arc::clone(&client_slot);
    std::thread::spawn(move || {
        if let Err(e) = Esp32MqttManager::run_loop(
            &config,
            wifi_ready_clone,
            mqtt_ready_clone,
            client_slot_clone,
        ) {
            log::error!("MQTT task failed: {e}");
        }
    });

    // Assemble components for FlowMonitor
    let pulse_counter = Esp32PulseCounter::new(flow_pin)?;
    let clock = Esp32Clock;
    let payload_builder = JsonPayloadBuilder {
        device_id: config.device_id,
    };
    let sink = MqttDataSink::new(client_slot, payload_builder, config.mqtt_topic);
    let guard = Esp32ConnectionGuard::new(wifi_ready, mqtt_ready);
    let delay = Esp32Delay;

    info!("System initialized, starting monitor...");

    // Run the main monitoring loop (never returns)
    run(pulse_counter, clock, sink, guard, delay)
}
