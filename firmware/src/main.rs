//! Firmware entry point.

#![allow(unused_imports)]

use anyhow::Result;

#[cfg(target_os = "espidf")]
use firmware::production_runner;
#[cfg(not(target_os = "espidf"))]
use firmware::{mock_runner, Event, EventKind};

fn main() -> Result<()> {
    #[cfg(target_os = "espidf")]
    {
        production_runner()
    }
    #[cfg(not(target_os = "espidf"))]
    {
        let events = mock_runner()?;

        // Print timeline
        println!("=== Event Timeline ===");
        for ev in &events {
            let t_s = ev.time_ms / 1000;
            let message = match &ev.kind {
                EventKind::SimulationStarted => "Simulation started".to_string(),
                EventKind::SimulationEnded => "Simulation ended".to_string(),
                EventKind::WifiUp => "WiFi connected".to_string(),
                EventKind::WifiDown => "WiFi disconnected!".to_string(),
                EventKind::MqttUp => "MQTT connected".to_string(),
                EventKind::MqttDown => "MQTT disconnected".to_string(),
                EventKind::SystemReady => "System ready (WiFi+MQTT)".to_string(),
                EventKind::SystemNotReady => "System not ready".to_string(),
                EventKind::SensorSample { pulses } => {
                    format!("Sensor sample taken ({} pulses)", pulses)
                }
                EventKind::PublishSuccess {
                    pulse_delta,
                    time_delta_ms,
                } => format!(
                    "Published sample ({} pulses, {} ms)",
                    pulse_delta, time_delta_ms
                ),
                EventKind::PublishFailure { reason } => {
                    format!("Publish failed: {}", reason)
                }
            };
            println!("[{:>4}s] {}", t_s, message);
        }

        // Summary
        let total_events = events.len();
        let publishes = events
            .iter()
            .filter(|e| matches!(e.kind, EventKind::PublishSuccess { .. }))
            .count();
        let sensor_samples = events
            .iter()
            .filter(|e| matches!(e.kind, EventKind::SensorSample { .. }))
            .count();
        let wifi_changes = events
            .iter()
            .filter(|e| matches!(e.kind, EventKind::WifiUp | EventKind::WifiDown))
            .count();
        let mqtt_changes = events
            .iter()
            .filter(|e| matches!(e.kind, EventKind::MqttUp | EventKind::MqttDown))
            .count();
        let failures = events
            .iter()
            .filter(|e| matches!(e.kind, EventKind::PublishFailure { .. }))
            .count();
        println!(
            "Summary: {} total events ({} sensor samples, {} publishes, {} WiFi changes, {} MQTT changes, {} failures)",
            total_events, sensor_samples, publishes, wifi_changes, mqtt_changes, failures
        );

        Ok(())
    }
}
