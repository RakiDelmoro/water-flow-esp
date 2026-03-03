use std::sync::atomic::{AtomicU32, Ordering};
use esp_idf_hal::gpio::{AnyIOPin, Input, InterruptType, PinDriver, Pull};
use crate::platform::traits::PulseCounter;

static PULSE_COUNT: AtomicU32 = AtomicU32::new(0);

pub struct Esp32PulseCounter {
    pin: PinDriver<'static, AnyIOPin, Input>,
}

impl Esp32PulseCounter {
    pub fn new(pin: AnyIOPin) -> anyhow::Result<Self> {
        PinDriver::input(pin)
            .map_err(anyhow::Error::from)
            .and_then(|mut p| {
                p.set_pull(Pull::Up)?;
                p.set_interrupt_type(InterruptType::NegEdge)?;
                Ok(Self { pin: p })
            })
    }
}

impl PulseCounter for Esp32PulseCounter {
    fn start(&mut self) -> anyhow::Result<()> {
        unsafe {
            self.pin.subscribe(|| {
                PULSE_COUNT.fetch_add(1, Ordering::Relaxed);
            })?;
        }
        self.pin.enable_interrupt().map_err(anyhow::Error::from)
    }

    fn enable_interrupt(&mut self) -> anyhow::Result<()> {
        self.pin.enable_interrupt().map_err(anyhow::Error::from)
    }

    fn total_pulses(&self) -> u32 { PULSE_COUNT.load(Ordering::Relaxed) }
    fn reset(&mut self)           { PULSE_COUNT.store(0, Ordering::Relaxed) }
}
