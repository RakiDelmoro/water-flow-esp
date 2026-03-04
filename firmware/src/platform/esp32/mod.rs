mod clock;
mod connection;
mod delay;
mod mqtt;
mod payload;
mod pulse_counter;
mod sampler;
mod wifi;

use anyhow::Result;
use esp_idf_hal::gpio::AnyIOPin;
use esp_idf_hal::gpio::Pins;

/// Helper to convert a GPIO pin number to an AnyIOPin.
/// Consumes the Pins struct and returns the requested pin.
pub fn take_pin(pins: Pins, pin_num: u8) -> Result<AnyIOPin> {
    match pin_num {
        0 => Ok(pins.gpio0.into()),
        1 => Ok(pins.gpio1.into()),
        2 => Ok(pins.gpio2.into()),
        3 => Ok(pins.gpio3.into()),
        4 => Ok(pins.gpio4.into()),
        5 => Ok(pins.gpio5.into()),
        6 => Ok(pins.gpio6.into()),
        7 => Ok(pins.gpio7.into()),
        8 => Ok(pins.gpio8.into()),
        9 => Ok(pins.gpio9.into()),
        10 => Ok(pins.gpio10.into()),
        11 => Ok(pins.gpio11.into()),
        12 => Ok(pins.gpio12.into()),
        13 => Ok(pins.gpio13.into()),
        14 => Ok(pins.gpio14.into()),
        15 => Ok(pins.gpio15.into()),
        16 => Ok(pins.gpio16.into()),
        17 => Ok(pins.gpio17.into()),
        18 => Ok(pins.gpio18.into()),
        19 => Ok(pins.gpio19.into()),
        21 => Ok(pins.gpio21.into()),
        22 => Ok(pins.gpio22.into()),
        23 => Ok(pins.gpio23.into()),
        25 => Ok(pins.gpio25.into()),
        26 => Ok(pins.gpio26.into()),
        27 => Ok(pins.gpio27.into()),
        32 => Ok(pins.gpio32.into()),
        33 => Ok(pins.gpio33.into()),
        _ => anyhow::bail!("Unsupported GPIO pin: {}", pin_num),
    }
}

pub use clock::Esp32Clock;
pub use connection::Esp32ConnectionGuard;
pub use delay::Esp32Delay;
pub use mqtt::{Esp32MqttManager, Esp32MqttPublisher};
pub use payload::JsonPayloadBuilder;
pub use pulse_counter::Esp32PulseCounter;
pub use sampler::HardwarePayloadSampler;
pub use wifi::Esp32WifiManager;
