mod connection_manager;
mod main_config;

use esp_idf_hal::gpio::{InterruptType, PinDriver, Pull};
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::task::queue::Queue;
use esp_idf_svc::mqtt::client::{EspMqttClient, QoS};
use esp_idf_svc::sys::configTICK_RATE_HZ;
use log::info;
use main_config::MQTT_TOPIC;
use serde_json::json;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

static PULSE_COUNT: AtomicU32 = AtomicU32::new(0);

// Raw pointer to the queue, set once at startup before ISR is enabled
static mut PULSE_QUEUE_PTR: Option<*mut Queue<u8>> = None;

fn time_now_in_millis() -> u64 {
    unsafe { (esp_idf_svc::sys::esp_timer_get_time() / 1000) as u64 }
}

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    info!("=== DEVICE POWERED ON - Starting initialization ===");
    info!("Device is powered by water flow - will run until water stops");

    let peripherals = Peripherals::take().expect("Failed to take peripherals");

    let wifi_connected = Arc::new(AtomicBool::new(false));
    let mqtt_connected = Arc::new(AtomicBool::new(false));
    let mqtt_first_connect = Arc::new(AtomicBool::new(true)); // true = waiting for first connect
    let mqtt_client: Arc<Mutex<Option<EspMqttClient<'static>>>> = Arc::new(Mutex::new(None));

    let wifi_connected_clone = Arc::clone(&wifi_connected);
    let mqtt_connected_clone = Arc::clone(&mqtt_connected);
    let mqtt_first_connect_clone = Arc::clone(&mqtt_first_connect);
    let mqtt_client_clone = Arc::clone(&mqtt_client);

    // Create FreeRTOS queue — ISR sends a byte per pulse, main loop blocks on it
    let pulse_queue = Queue::<u8>::new(1);
    unsafe {
        PULSE_QUEUE_PTR = Some(&pulse_queue as *const Queue<u8> as *mut Queue<u8>);
    }

    // Setup flow sensor — ISR increments counter AND signals the queue
    let mut flow_pin = PinDriver::input(peripherals.pins.gpio25)?;
    flow_pin.set_pull(Pull::Up)?;
    flow_pin.set_interrupt_type(InterruptType::AnyEdge)?;
    unsafe {
        flow_pin.subscribe(|| {
            PULSE_COUNT.fetch_add(1, Ordering::Relaxed);
            // Signal the main loop that a pulse happened (non-blocking, ISR-safe)
            if let Some(queue) = PULSE_QUEUE_PTR {
                (*queue).send_back(1, 0).ok();
            }
        })?;
    }
    flow_pin.enable_interrupt()?;
    info!("Flow sensor reading started on GPIO 25 - counting pulses immediately");

    // Spawn connection thread — WiFi + MQTT, runs in parallel with sensor reading
    let _connection_thread = std::thread::Builder::new()
        .stack_size(8192)
        .spawn(move || {
            if let Err(e) = connection_manager::run_connection_loop(
                peripherals.modem,
                wifi_connected_clone,
                mqtt_connected_clone,
                mqtt_first_connect_clone,
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
        // Block until a pulse arrives OR 1 second timeout
        // Thread sleeps here — no polling, no busy loop, watchdog happy
        let timeout_ticks = (1000 * configTICK_RATE_HZ) / 1000;
        if pulse_queue.recv_front(timeout_ticks).is_some() {
            flow_pin.enable_interrupt()?;
        }

        // On first MQTT connect: publish all accumulated pulses, then start clean 1s intervals
        if mqtt_first_connect.load(Ordering::Relaxed) && mqtt_connected.load(Ordering::Relaxed) {
            if let Ok(mut client_guard) = mqtt_client.try_lock() {
                if let Some(ref mut client) = client_guard.as_mut() {
                    let now = time_now_in_millis();
                    let pulses = PULSE_COUNT.load(Ordering::Relaxed);
                    let time_delta = now - last_sample_time;
                    let pulse_delta = pulses.saturating_sub(last_pulse_count);

                    // Publish the accumulated boot data
                    let payload = json!({
                        "total_pulses": pulse_delta,
                        "Time_ms": time_delta,
                        "accumulative_pulses": pulse_delta,
                        "first_connect": true
                    });

                    match client.publish(
                        MQTT_TOPIC,
                        QoS::AtLeastOnce,
                        false,
                        payload.to_string().as_bytes(),
                    ) {
                        Ok(_) => {
                            info!("First MQTT publish: {} pulses over {}ms", pulse_delta, time_delta);
                            last_pulse_count = pulses;
                            last_sample_time = now;
                            mqtt_first_connect.store(false, Ordering::Relaxed);
                        }
                        Err(e) => {
                            info!("Failed to publish first data: {:?}", e);
                        }
                    }
                }
            }
            continue;
        }

        // Normal 1-second publish cycle
        if wifi_connected.load(Ordering::Relaxed) && mqtt_connected.load(Ordering::Relaxed)
            && !mqtt_first_connect.load(Ordering::Relaxed)
            && time_now_in_millis() - last_sample_time >= 1_000
        {
            let now = time_now_in_millis();
            let pulses = PULSE_COUNT.load(Ordering::Relaxed);

            if let Ok(mut client_guard) = mqtt_client.try_lock() {
                if let Some(ref mut client) = client_guard.as_mut() {
                    if mqtt_connected.load(Ordering::Relaxed) {
                        let time_delta = now - last_sample_time;
                        let pulse_delta = pulses.saturating_sub(last_pulse_count);
                        let payload = json!({"total_pulses": pulse_delta, "Time_ms": time_delta, "accumulative_pulses": pulses});

                        match client.publish(
                            MQTT_TOPIC,
                            QoS::AtLeastOnce,
                            false,
                            payload.to_string().as_bytes(),
                        ) {
                            Ok(_) => {
                                last_pulse_count = pulses;
                                last_sample_time = now;
                            }
                            Err(e) => {
                                info!("Failed to publish data: {:?}", e);
                            }
                        }
                    }
                }
            }
        }
    }
}