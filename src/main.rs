mod connection_manager;
mod main_config;

use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::{InterruptType, PinDriver, Pull};
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::mqtt::client::{EspMqttClient, QoS};
use log::info;
use main_config::MQTT_TOPIC;
use serde_json::json;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

// `static` creates a single global value with a fixed memory address.
// Unlike `const`, it is not inlined and can be mutated (here safely via `AtomicU32`).
static PULSE_COUNT: AtomicU32 = AtomicU32::new(0);

fn time_now_in_millis() -> u64 {
    unsafe { (esp_idf_svc::sys::esp_timer_get_time() / 1000) as u64 }
}

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    info!("=== DEVICE POWERED ON - Starting initialization ===");
    info!("Device is powered by water flow - will run until water stops");

    let peripherals = Peripherals::take().expect("Failed to take peripherals");

    // Wrap in Arc so they can be shared across threads
    let wifi_connected = Arc::new(AtomicBool::new(false));
    let mqtt_connected = Arc::new(AtomicBool::new(false));
    let mqtt_client: Arc<Mutex<Option<EspMqttClient<'static>>>> = Arc::new(Mutex::new(None));

    // Clone Arc references for the connection thread
    let wifi_connected_clone = Arc::clone(&wifi_connected);
    let mqtt_connected_clone = Arc::clone(&mqtt_connected);
    let mqtt_client_clone = Arc::clone(&mqtt_client);

    // Setup flow sensor - starts counting pulses immediately
    let mut flow_pin = PinDriver::input(peripherals.pins.gpio25)?;
    flow_pin.set_pull(Pull::Up)?;
    flow_pin.set_interrupt_type(InterruptType::AnyEdge)?;
    unsafe {
        flow_pin.subscribe(|| {
            PULSE_COUNT.fetch_add(1, Ordering::Relaxed);
        })?;
    }
    info!("Flow sensor reading started on GPIO 25 - counting pulses immediately");

    // Spawn single connection management thread (WiFi + MQTT)
    let _connection_thread = std::thread::Builder::new()
        .stack_size(8192)
        .spawn(move || {
            if let Err(e) = connection_manager::run_connection_loop(
                peripherals.modem,
                wifi_connected_clone,
                mqtt_connected_clone,
                mqtt_client_clone,
            ) {
                info!("Connection thread error: {:?}", e);
            }
        })?;

    info!("=== Initialization complete - entering main loop ===");
    info!("Sensor reading continues regardless of WiFi/MQTT state");

    let mut last_sample_time = time_now_in_millis();
    let mut last_pulse_count: u32 = PULSE_COUNT.load(Ordering::Relaxed);
    loop {
        flow_pin.enable_interrupt()?; // Start accumulate
        if time_now_in_millis() - last_sample_time < 1_000 {
            FreeRtos::delay_ms(10); // Prevent busy loop and watchdog timeout
            continue;
        }

        let now = time_now_in_millis();
        let pulses = PULSE_COUNT.load(Ordering::Relaxed);

        if !wifi_connected.load(Ordering::Relaxed) || !mqtt_connected.load(Ordering::Relaxed) {
            FreeRtos::delay_ms(100);
            continue;
        }

        if let Ok(mut client_guard) = mqtt_client.try_lock() {
            if let Some(ref mut client) = client_guard.as_mut() {
                if !mqtt_connected.load(Ordering::Relaxed) {
                    continue;
                }
                let time_delta = now - last_sample_time;
                let pulse_delta = pulses.saturating_sub(last_pulse_count);
                let payload = json!({"total_pulses": pulse_delta, "Time_ms": time_delta, "accumulative_pulses": last_pulse_count});

                match client.publish(
                    MQTT_TOPIC,
                    QoS::AtLeastOnce,
                    false,
                    payload.to_string().as_bytes(),
                ) {
                    Ok(_) => {
                        last_pulse_count += pulse_delta;
                        last_sample_time += time_delta;
                    }
                    Err(e) => {
                        info!("Failed to publish data: {:?}", e);
                    }
                }
            }
        }
    }
}
