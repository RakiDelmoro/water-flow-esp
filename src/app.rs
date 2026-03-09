//! Application Logic Module
//!
//! Event-driven main loop and MQTT publishing.

use crate::main_config::MQTT_TOPIC;
use esp_idf_hal::gpio::PinDriver;
use esp_idf_svc::mqtt::client::{EspMqttClient, QoS};
use log::{debug, error, info, warn};
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc::Receiver, Arc, Mutex};

use crate::sensor::get_pulse_count;
use crate::timer::now_ms;
use crate::AppEvent;

/// Run event-driven main loop
///
/// Blocks on events from channel - no busy-waiting.
/// Processes timer ticks to sample and publish data.
pub fn run_event_loop(event_rx: &Receiver<AppEvent>, flow_pin: &mut PinDriver<esp_idf_hal::gpio::Gpio25, esp_idf_hal::gpio::Input>, wifi_connected: &Arc<AtomicBool>, mqtt_connected: &Arc<AtomicBool>, mqtt_client: &Arc<Mutex<Option<EspMqttClient<'static>>>>) -> anyhow::Result<()> {
    let mut last_pulse_count: u32 = 0;
    let mut last_sample_time: u64 = now_ms();

    loop {
        // Enable interrupts
        flow_pin.enable_interrupt()?;

        // Block on event (no busy-wait)
        match event_rx.recv() {
            Ok(AppEvent::TimerTick) => {
                let now = now_ms();
                let pulses = get_pulse_count();
                let pulse_delta = pulses.saturating_sub(last_pulse_count);

                if pulse_delta > 0 {
                    debug!(target: "app", "Sample: {} pulses", pulse_delta);
                }

                // Try to publish if network ready
                let wifi_ready = wifi_connected.load(Ordering::Relaxed);
                let mqtt_ready = mqtt_connected.load(Ordering::Relaxed);

                if wifi_ready && mqtt_ready {
                    let time_delta = now.saturating_sub(last_sample_time);
                    if let Err(e) =
                        publish_data(mqtt_client, pulse_delta, last_pulse_count, time_delta)
                    {
                        warn!(target: "app", "Publish failed: {:?}", e);
                    } else {
                        // Update tracking on success
                        last_pulse_count = pulses;
                        last_sample_time = now;
                    }
                } else {
                    debug!(target: "app", "Network not ready (wifi={}, mqtt={})", wifi_ready, mqtt_ready);
                }
            }
            Ok(AppEvent::WifiState(connected)) => {
                info!(target: "app", "WiFi {}", if connected { "connected" } else { "disconnected" });
            }
            Ok(AppEvent::MqttState(connected)) => {
                info!(target: "app", "MQTT {}", if connected { "connected" } else { "disconnected" });
            }
            Err(e) => {
                error!(target: "app", "Event channel error: {:?}", e);
                break;
            }
        }
    }

    Ok(())
}

/// Publish pulse data to MQTT
///
/// Fast-fail: try once, log error, move on.
fn publish_data(mqtt_client: &Arc<Mutex<Option<EspMqttClient<'static>>>>, pulse_delta: u32, accumulative_pulses: u32, time_ms: u64) -> anyhow::Result<()> {
    let payload = json!({
        "pulse_delta": pulse_delta,
        "time_ms": time_ms,
        "accumulative_pulses": accumulative_pulses,
    });

    let mut client_guard = mqtt_client
        .lock()
        .map_err(|_| anyhow::anyhow!("Lock poisoned"))?;

    let client = client_guard
        .as_mut()
        .ok_or_else(|| anyhow::anyhow!("Not connected"))?;

    client.publish(
        MQTT_TOPIC,
        QoS::AtLeastOnce,
        false,
        payload.to_string().as_bytes(),
    )?;

    if pulse_delta > 0 {
        info!(target: "publish", "{} pulses in {}ms", pulse_delta, time_ms);
    }

    Ok(())
}
