//! Firmware library - core logic and abstractions.
//!
//! This crate provides:
//! - `engine`: The FlowMonitor orchestrator
//! - `platform`: Platform abstractions (traits) and implementations
//! - `run()`: Top-level function to start the monitoring system

pub mod config;
pub mod engine;
pub mod platform;

use crate::engine::FlowMonitor;
use crate::platform::traits::{Clock, ConnectionGuard, DataSink, Delay, PulseCounter};

/// Start the flow monitoring system.
///
/// Takes ownership of the five required components and runs the monitoring
/// loop indefinitely. The loop sends flow data to the MQTT broker when
/// both WiFi and MQTT are connected.
pub fn run<P, C, S, G, D>(
    pulse_counter: P,
    clock: C,
    sink: S,
    guard: G,
    delay: D,
) -> anyhow::Result<()>
where
    P: PulseCounter,
    C: Clock,
    S: DataSink,
    G: ConnectionGuard,
    D: Delay,
{
    let mut monitor = FlowMonitor::new(pulse_counter, clock, sink, guard, delay)?;
    monitor.start()
}
