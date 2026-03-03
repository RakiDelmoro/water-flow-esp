use anyhow::Result;
use std::sync::atomic::{AtomicU32, Ordering};

// Global pulse counter - safe for ISR and main thread
static PULSE_COUNT: AtomicU32 = AtomicU32::new(0);

// Interrupt Service Routine - called on each flow pulse
#[cfg(target_os = "espidf")]
unsafe extern "C" fn flow_sensor_isr(_arg: *mut core::ffi::c_void) {
    PULSE_COUNT.fetch_add(1, Ordering::Relaxed);
}

/// Production implementation of FlowCounter using ESP hardware
pub struct EspFlowCounter;

impl crate::traits::FlowCounter for EspFlowCounter {
    fn swap(&self, reset: bool) -> u32 {
        if reset {
            PULSE_COUNT.swap(0, Ordering::Relaxed)
        } else {
            PULSE_COUNT.load(Ordering::Relaxed)
        }
    }
}

#[cfg(target_os = "espidf")]
/// Configures the flow sensor GPIO interrupt based on FLOW_SENSOR_PIN
/// Takes ownership of the Pins struct and consumes the selected pin.
pub fn setup_flow_sensor(pins: esp_idf_hal::gpio::Pins) -> Result<()> {
    use esp_idf_hal::gpio::{
        enable_isr_service, InputPin, InterruptType, OutputPin, PinDriver, Pull,
    };
    use esp_idf_svc::sys;

    match crate::config::FLOW_SENSOR_PIN {
        4 => configure_pin(pins.gpio4, 4)?,
        5 => configure_pin(pins.gpio5, 5)?,
        12 => configure_pin(pins.gpio12, 12)?,
        13 => configure_pin(pins.gpio13, 13)?,
        14 => configure_pin(pins.gpio14, 14)?,
        15 => configure_pin(pins.gpio15, 15)?,
        16 => configure_pin(pins.gpio16, 16)?,
        17 => configure_pin(pins.gpio17, 17)?,
        18 => configure_pin(pins.gpio18, 18)?,
        19 => configure_pin(pins.gpio19, 19)?,
        21 => configure_pin(pins.gpio21, 21)?,
        22 => configure_pin(pins.gpio22, 22)?,
        23 => configure_pin(pins.gpio23, 23)?,
        25 => configure_pin(pins.gpio25, 25)?,
        26 => configure_pin(pins.gpio26, 26)?,
        27 => configure_pin(pins.gpio27, 27)?,
        32 => configure_pin(pins.gpio32, 32)?,
        33 => configure_pin(pins.gpio33, 33)?,
        _ => {
            return Err(anyhow!(
                "Unsupported flow sensor pin: {}",
                crate::config::FLOW_SENSOR_PIN
            ))
        }
    }

    Ok(())
}

#[cfg(target_os = "espidf")]
fn configure_pin<T: InputPin + OutputPin>(pin: T, pin_num: i32) -> Result<()> {
    use esp_idf_hal::gpio::PinDriver;
    use esp_idf_svc::sys;

    let mut pin_driver = PinDriver::input(pin)?;
    pin_driver.set_pull(esp_idf_hal::gpio::Pull::Up)?;
    pin_driver.set_interrupt_type(esp_idf_hal::gpio::InterruptType::PosEdge)?;

    unsafe {
        esp_idf_hal::gpio::enable_isr_service()?;
        sys::gpio_isr_handler_add(
            pin_num,
            Some(flow_sensor_isr),
            pin_num as *mut core::ffi::c_void,
        );
        sys::gpio_intr_enable(pin_num);
    }

    // Prevent pin from being dropped - interrupt will remain active
    std::mem::forget(pin_driver);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::FlowCounter;
    use std::sync::atomic::Ordering;

    #[test]
    fn flow_counter_swap_loads_count() {
        PULSE_COUNT.store(0, Ordering::Relaxed);

        let counter = EspFlowCounter;
        let count = counter.swap(false);
        assert_eq!(count, 0);
    }

    #[test]
    fn flow_counter_swap_resets_count() {
        PULSE_COUNT.store(0, Ordering::Relaxed);

        let counter = EspFlowCounter;
        PULSE_COUNT.store(42, Ordering::Relaxed);

        let old_count = counter.swap(true);
        assert_eq!(old_count, 42);
        assert_eq!(PULSE_COUNT.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn counter_swap_without_reset_preserves_count() {
        PULSE_COUNT.store(100, Ordering::Relaxed);

        let counter = EspFlowCounter;
        let count = counter.swap(false);
        assert_eq!(count, 100);
        assert_eq!(PULSE_COUNT.load(Ordering::Relaxed), 100);
    }

    #[test]
    fn counter_handles_multiple_swaps() {
        PULSE_COUNT.store(0, Ordering::Relaxed);

        let counter = EspFlowCounter;

        PULSE_COUNT.fetch_add(10, Ordering::Relaxed);
        PULSE_COUNT.fetch_add(20, Ordering::Relaxed);

        assert_eq!(counter.swap(false), 30);
        assert_eq!(PULSE_COUNT.load(Ordering::Relaxed), 30); // still 30
                                                             // After a reset it becomes 0
        assert_eq!(counter.swap(true), 30);
        assert_eq!(PULSE_COUNT.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn pulse_count_atomicity() {
        PULSE_COUNT.store(0, Ordering::Relaxed);

        for _ in 0..100 {
            PULSE_COUNT.fetch_add(1, Ordering::Relaxed);
        }

        let counter = EspFlowCounter;
        assert_eq!(counter.swap(true), 100);
    }
}
