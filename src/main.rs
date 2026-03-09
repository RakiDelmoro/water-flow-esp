//! Water Flow Sensor - Event-Driven Architecture
//!
//! Self-powered device: no busy-waiting, efficient power usage.
//! Uses hardware timer + channels for event-driven sampling.

mod app;
mod main_config;
mod mqtt_manager;
mod sensor;
mod timer;
mod wifi_manager;

use esp_idf_hal::peripherals::Peripherals;
use log::{info, warn};
use std::sync::atomic::AtomicBool;
use std::sync::{mpsc, Arc, Mutex};
use crate::sensor::setup_flow_sensor;
use crate::timer::SampleTimer;

/// Events for event-driven main loop
#[derive(Debug, Clone, Copy)]
pub enum AppEvent {
    TimerTick,       // 1-second timer fired
    WifiState(bool), // WiFi connected/disconnected
    MqttState(bool), // MQTT connected/disconnected
}

fn main() -> anyhow::Result<()> {
    // Initialize ESP-IDF
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    info!(target: "init", "=== Water Flow Sensor Starting ===");

    // Setup
    let peripherals = Peripherals::take().expect("Failed to take peripherals");
    let (event_tx, event_rx) = mpsc::channel::<AppEvent>();
    let event_tx = Arc::new(Mutex::new(event_tx));

    // Shared state
    let wifi_connected = Arc::new(AtomicBool::new(false));
    let mqtt_connected = Arc::new(AtomicBool::new(false));
    let mqtt_client: Arc<Mutex<Option<esp_idf_svc::mqtt::client::EspMqttClient<'static>>>> =
        Arc::new(Mutex::new(None));

    // Sensor
    let mut flow_pin = setup_flow_sensor(peripherals.pins.gpio25)?;

    // WiFi thread
    let wifi = wifi_manager::setup_wifi(peripherals.modem)?;
    let _wifi_thread = std::thread::Builder::new().stack_size(4096).spawn({
        let tx = Arc::clone(&event_tx);
        let wifi_conn = Arc::clone(&wifi_connected);
        move || {
            if let Err(e) = wifi_manager::run_wifi_loop(wifi, wifi_conn, tx) {
                warn!(target: "wifi", "Error: {:?}", e);
            }
        }
    })?;

    // MQTT thread
    let _mqtt_thread = std::thread::Builder::new().stack_size(4096).spawn({
        let tx = Arc::clone(&event_tx);
        let mqtt_conn = Arc::clone(&mqtt_connected);
        let client = Arc::clone(&mqtt_client);
        move || {
            if let Err(e) = mqtt_manager::run_mqtt_loop(mqtt_conn, client, tx) {
                warn!(target: "mqtt", "Error: {:?}", e);
            }
        }
    })?;

    // Timer
    let _timer = SampleTimer::new(event_tx)?;

    // Run event loop
    info!(target: "init", "=== Entering event loop ===");
    app::run_event_loop(&event_rx, &mut flow_pin, &wifi_connected, &mqtt_connected, &mqtt_client)
}
