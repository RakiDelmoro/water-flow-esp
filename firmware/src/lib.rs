//! Firmware library - core logic and abstractions.
//!
//! This crate provides:
//! - `engine`: The FlowMonitor orchestrator
//! - `platform`: Platform abstractions (traits) and implementations
//! - `run()`: Top-level function to start the monitoring system

pub mod config;
pub mod engine;
pub mod platform;

use crate::engine::Engine;
use crate::platform::traits::{Clock, ConnectionGuard, DataSink, Delay, PulseCounter};

/// Simulation event types for mock runner debugging/output.
#[derive(Debug)]
pub enum EventKind {
    SimulationStarted,
    SimulationEnded,
    WifiUp,
    WifiDown,
    MqttUp,
    MqttDown,
    SystemReady,
    SystemNotReady,
    SensorSample {
        pulses: u32,
    },
    PublishSuccess {
        pulse_delta: u32,
        time_delta_ms: u64,
    },
    PublishFailure {
        reason: String,
    },
}

#[derive(Debug)]
pub struct Event {
    pub time_ms: u64,
    pub kind: EventKind,
}

/// Start the flow monitoring system.
///
/// Takes ownership of the five required components and runs the monitoring
/// loop indefinitely. The loop sends flow data to the MQTT broker when
/// both WiFi and MQTT are connected.
pub fn run<P, C, S, G, D, F>(
    pulse_counter: P,
    clock: C,
    sink: S,
    guard: G,
    delay: D,
    tick_hook: F,
    max_ticks: Option<usize>,
) -> anyhow::Result<()>
where
    P: PulseCounter,
    C: Clock,
    S: DataSink,
    G: ConnectionGuard,
    D: Delay,
    F: FnMut(&mut Engine<P, C, S, G, D>, usize) -> anyhow::Result<()>,
{
    let mut engine = Engine::new(pulse_counter, clock, sink, guard, delay)?;
    engine.run_loop(tick_hook, max_ticks)
}

// ============================================================================
// Entry Points
// ============================================================================

/// Production entry point for ESP32 hardware.
///
/// Sets up WiFi/MQTT threads, hardware pulse counter, and runs the monitor.
/// This function only compiles for `target_os = "espidf"`.
#[cfg(target_os = "espidf")]
pub fn production_runner() -> anyhow::Result<()> {
    use crate::config::Config;
    use crate::platform::esp32::*;
    use crate::platform::sink::MqttDataSink;
    use crate::platform::traits::{MqttManager, WifiManager};
    use crate::run;
    use esp_idf_hal::{gpio::Pins, modem::Modem};
    use esp_idf_svc::log::EspLogger;
    use log::info;
    use std::sync::{atomic::AtomicBool, Arc, Mutex};

    let config = Config::from_env()?;
    EspLogger::initialize_default();
    info!("Starting water flow monitor...");

    let modem = unsafe { Modem::new() };
    let pins = unsafe { Pins::new() };
    let flow_pin = take_pin(pins, config.flow_sensor_pin)?;

    let wifi_ready = Arc::new(AtomicBool::new(false));
    let mqtt_ready = Arc::new(AtomicBool::new(false));
    let client_slot = Arc::new(Mutex::new(None));

    // Spawn WiFi manager thread
    let wifi_ready_clone = Arc::clone(&wifi_ready);
    let wifi_manager = Esp32WifiManager::setup(modem, &config.wifi_ssid, &config.wifi_pass)?;
    std::thread::spawn(move || {
        if let Err(e) = wifi_manager.run_loop(wifi_ready_clone, None) {
            log::error!("WiFi task failed: {e}");
        }
    });

    // Spawn MQTT manager thread
    let wifi_ready_clone = Arc::clone(&wifi_ready);
    let mqtt_ready_clone = Arc::clone(&mqtt_ready);
    let client_slot_clone = Arc::clone(&client_slot);
    // Clone needed data before moving config into the thread
    let mqtt_topic = config.mqtt_topic.clone();
    let device_id = config.device_id.clone();
    std::thread::spawn(move || {
        if let Err(e) = Esp32MqttManager::run_loop(
            &config,
            wifi_ready_clone,
            mqtt_ready_clone,
            client_slot_clone,
            None,
        ) {
            log::error!("MQTT task failed: {e}");
        }
    });

    // Assemble components for FlowMonitor
    let pulse_counter = Esp32PulseCounter::new(flow_pin)?;
    let clock = Esp32Clock;
    let payload_builder = JsonPayloadBuilder { device_id };
    let sink = MqttDataSink::new(client_slot, payload_builder, mqtt_topic);
    let guard = Esp32ConnectionGuard::new(wifi_ready, mqtt_ready);
    let delay = Esp32Delay;

    info!("Production mode...");

    // Run the main monitoring loop (never returns)
    run(pulse_counter, clock, sink, guard, delay, |engine, _| engine.tick(), None)
}

/// Mock runner for host development and testing.
///
/// Creates deterministic mock components and runs a finite simulation with
/// WiFi and MQTT manager threads (mirroring production architecture).
/// Returns a timeline of events that occurred during simulation.
/// This function only compiles for non-ESP32 targets.
#[cfg(not(target_os = "espidf"))]
pub fn mock_runner() -> anyhow::Result<Vec<Event>> {
    use crate::config::Config;
    use crate::platform::host::*;
    use crate::platform::sink::MqttDataSink;
    use crate::platform::traits::{MqttManager, WifiManager};
    use std::env;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    // Read simulation parameters
    let ticks: usize = env::var("MOCK_TICKS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);
    let connect_at: Option<usize> = env::var("MOCK_CONNECT_AT")
        .ok()
        .and_then(|s| s.parse().ok())
        .or(Some(1));
    let disconnect_at: Option<usize> = env::var("MOCK_DISCONNECT_AT")
        .ok()
        .and_then(|s| s.parse().ok());
    let reconnect_at: Option<usize> = env::var("MOCK_RECONNECT_AT")
        .ok()
        .and_then(|s| s.parse().ok());

    // Shared state
    let wifi_ready = Arc::new(AtomicBool::new(false));
    let mqtt_ready = Arc::new(AtomicBool::new(false));
    let client_slot: Arc<Mutex<Option<HostMqttPublisher>>> = Arc::new(Mutex::new(None));
    let shutdown = Arc::new(AtomicBool::new(false));

    // Event tracking
    let mut events = Vec::new();
    events.push(Event {
        time_ms: 0,
        kind: EventKind::SimulationStarted,
    });

    let mut prev_wifi = wifi_ready.load(Ordering::Relaxed);
    let mut prev_mqtt = mqtt_ready.load(Ordering::Relaxed);
    let mut prev_ready = prev_wifi && prev_mqtt;

    // Managers
    let wifi_manager = HostWifiManager::new(false);

    // Spawn WiFi thread
    let wifi_ready_c = Arc::clone(&wifi_ready);
    let shutdown_c = Arc::clone(&shutdown);
    let wifi_thread = thread::spawn(move || {
        wifi_manager
            .run_loop(wifi_ready_c, Some(shutdown_c))
            .unwrap_or_else(|e| eprintln!("WiFi thread error: {e}"));
    });

    // Spawn MQTT thread
    let wifi_ready_c = Arc::clone(&wifi_ready);
    let mqtt_ready_c = Arc::clone(&mqtt_ready);
    let client_slot_c = Arc::clone(&client_slot);
    let config = Config::default();
    let shutdown_c = Arc::clone(&shutdown);
    let mqtt_thread = thread::spawn(move || {
        HostMqttManager::run_loop(
            &config,
            wifi_ready_c,
            mqtt_ready_c,
            client_slot_c,
            Some(shutdown_c),
        )
        .unwrap_or_else(|e| eprintln!("MQTT thread error: {e}"));
    });

    // Give threads a moment to initialize
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Engine components
    let pulse_counter = HostPulseCounter::new();
    let clock = HostClock::new(0);
    let payload_builder = HostPayloadBuilder;
    let sink = MqttDataSink::new(
        client_slot.clone(),
        payload_builder,
        "water/flow".to_string(),
    );
    let guard = HostConnectionGuard::new_with_arcs(Arc::clone(&wifi_ready), Arc::clone(&mqtt_ready));
    let delay = HostDelay::new();

    let mut final_time = 0u64;
    
    run(pulse_counter, clock, sink, guard, delay,
        |engine, tick_num| {
            // WiFi state triggers
            if let Some(at) = connect_at {
                if tick_num + 1 == at {
                    wifi_ready.store(true, Ordering::Relaxed);
                }
            }
            if let Some(at) = disconnect_at {
                if tick_num + 1 == at {
                    wifi_ready.store(false, Ordering::Relaxed);
                }
            }
            if let Some(at) = reconnect_at {
                if tick_num + 1 == at {
                    wifi_ready.store(true, Ordering::Relaxed);
                }
            }

            // Simulate time passing and pulses
            engine.clock.advance(1000);
            engine.pulse_counter.inject_pulses(10);

            let now_ms = engine.clock.time_now_in_millis();
            let wifi = wifi_ready.load(Ordering::Relaxed);
            let mqtt = mqtt_ready.load(Ordering::Relaxed);
            let ready = wifi && mqtt;

            // Track connection state changes
            if wifi != prev_wifi {
                events.push(Event {
                    time_ms: now_ms,
                    kind: if wifi {
                        EventKind::WifiUp
                    } else {
                        EventKind::WifiDown
                    },
                });
            }
            if mqtt != prev_mqtt {
                events.push(Event {
                    time_ms: now_ms,
                    kind: if mqtt {
                        EventKind::MqttUp
                    } else {
                        EventKind::MqttDown
                    },
                });
            }
            if ready != prev_ready {
                events.push(Event {
                    time_ms: now_ms,
                    kind: if ready {
                        EventKind::SystemReady
                    } else {
                        EventKind::SystemNotReady
                    },
                });
            }
            prev_wifi = wifi;
            prev_mqtt = mqtt;
            prev_ready = ready;

            // Sensor sample and publish
            let elapsed_ms = now_ms - engine.last_sample_time;
            if elapsed_ms >= 1000 {
                // Log that we took a sample
                let current_pulses = engine.pulse_counter.total_pulses();
                events.push(Event {
                    time_ms: now_ms,
                    kind: EventKind::SensorSample {
                        pulses: current_pulses,
                    },
                });

                if ready {
                    let before_total = engine.last_pulse_count;
                    let before_time = engine.last_sample_time;
                    // Shared tick() call
                    match engine.tick() {
                        Ok(()) => {
                            let delta = engine.last_pulse_count - before_total;
                            let time_delta = now_ms - before_time;
                            events.push(Event {
                                time_ms: now_ms,
                                kind: EventKind::PublishSuccess {
                                    pulse_delta: delta,
                                    time_delta_ms: time_delta,
                                },
                            });
                        }
                        Err(e) => {
                            events.push(Event {
                                time_ms: now_ms,
                                kind: EventKind::PublishFailure {
                                    reason: e.to_string(),
                                },
                            });
                        }
                    }
                } else {
                    let _ = engine.tick();
                }
            }

            thread::sleep(Duration::from_millis(10));
            final_time = now_ms;
            Ok(())
        },
        Some(ticks),
    )?;

    // Shutdown
    events.push(Event {
        time_ms: final_time,
        kind: EventKind::SimulationEnded,
    });
    shutdown.store(true, Ordering::Relaxed);

    // Join threads
    wifi_thread
        .join()
        .map_err(|e| anyhow::anyhow!("WiFi thread join failed: {:?}", e))?;
    mqtt_thread
        .join()
        .map_err(|e| anyhow::anyhow!("MQTT thread join failed: {:?}", e))?;

    Ok(events)
}
