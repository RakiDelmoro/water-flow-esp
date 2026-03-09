//! Flow Sensor Module
//!
//! Handles GPIO interrupt setup and pulse counting.

use esp_idf_hal::gpio::{InterruptType, PinDriver, Pull};
use std::sync::atomic::{AtomicU32, Ordering};

/// Global pulse counter - incremented by GPIO interrupt
pub static PULSE_COUNT: AtomicU32 = AtomicU32::new(0);

/// Setup flow sensor with GPIO interrupt
///
/// Configures pin as input with pull-up and edge-triggered interrupt.
/// Pulses are counted via atomic counter (non-blocking).
pub fn setup_flow_sensor(pin: esp_idf_hal::gpio::Gpio25) -> anyhow::Result<PinDriver<'static, esp_idf_hal::gpio::Gpio25, esp_idf_hal::gpio::Input>> {
    let mut flow_pin = PinDriver::input(pin)?;
    flow_pin.set_pull(Pull::Up)?;
    flow_pin.set_interrupt_type(InterruptType::AnyEdge)?;

    unsafe {
        flow_pin.subscribe(|| {
            PULSE_COUNT.fetch_add(1, Ordering::Relaxed);
        })?;
    }

    Ok(flow_pin)
}

/// Get current pulse count
pub fn get_pulse_count() -> u32 {
    PULSE_COUNT.load(Ordering::Relaxed)
}
