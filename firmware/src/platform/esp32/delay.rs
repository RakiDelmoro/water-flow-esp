use crate::platform::traits::Delay;

pub struct Esp32Delay;

impl Delay for Esp32Delay {
    fn delay_ms(&self, ms: u32) {
        std::thread::sleep(std::time::Duration::from_millis(ms as u64))
    }
}
