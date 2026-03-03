mod clock;
mod connection;
mod delay;
mod mqtt;
mod payload;
mod pulse_counter;
mod sampler;
mod sink;
mod wifi;

use anyhow::Result;
use esp_idf_hal::gpio::AnyIOPin;
use esp_idf_hal::Pins;

/// Helper to convert a GPIO pin number to an AnyIOPin.
/// Consumes the Pins struct and returns the requested pin.
pub fn take_pin(pins: Pins, pin_num: u8) -> Result<AnyIOPin> {
    match pin_num {
        0 => Ok(pins.gpio0.into_any_io()),
        1 => Ok(pins.gpio1.into_any_io()),
        2 => Ok(pins.gpio2.into_any_io()),
        3 => Ok(pins.gpio3.into_any_io()),
        4 => Ok(pins.gpio4.into_any_io()),
        5 => Ok(pins.gpio5.into_any_io()),
        6 => Ok(pins.gpio6.into_any_io()),
        7 => Ok(pins.gpio7.into_any_io()),
        8 => Ok(pins.gpio8.into_any_io()),
        9 => Ok(pins.gpio9.into_any_io()),
        10 => Ok(pins.gpio10.into_any_io()),
        11 => Ok(pins.gpio11.into_any_io()),
        12 => Ok(pins.gpio12.into_any_io()),
        13 => Ok(pins.gpio13.into_any_io()),
        14 => Ok(pins.gpio14.into_any_io()),
        15 => Ok(pins.gpio15.into_any_io()),
        16 => Ok(pins.gpio16.into_any_io()),
        17 => Ok(pins.gpio17.into_any_io()),
        18 => Ok(pins.gpio18.into_any_io()),
        19 => Ok(pins.gpio19.into_any_io()),
        21 => Ok(pins.gpio21.into_any_io()),
        22 => Ok(pins.gpio22.into_any_io()),
        23 => Ok(pins.gpio23.into_any_io()),
        25 => Ok(pins.gpio25.into_any_io()),
        26 => Ok(pins.gpio26.into_any_io()),
        27 => Ok(pins.gpio27.into_any_io()),
        32 => Ok(pins.gpio32.into_any_io()),
        33 => Ok(pins.gpio33.into_any_io()),
        _ => anyhow::bail!("Unsupported GPIO pin: {}", pin_num),
    }
}

pub use clock::Esp32Clock;
pub use connection::Esp32ConnectionGuard;
pub use delay::Esp32Delay;
pub use mqtt::{Esp32MqttManager, Esp32MqttPublisher};
pub use payload::JsonPayloadBuilder;
pub use pulse_counter::Esp32PulseCounter;
pub use sampler::HardwareFlowSampler;
pub use sink::MqttDataSink;
pub use wifi::Esp32WifiManager;
