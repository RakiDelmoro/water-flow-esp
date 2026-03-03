use std::sync::{Arc, Mutex};
use crate::platform::traits::PulseCounter;

#[derive(Clone)]
pub struct HostPulseCounter {
    pulses:            Arc<Mutex<u32>>,
    started:           Arc<Mutex<bool>>,
    interrupt_enabled: Arc<Mutex<bool>>,
}

impl HostPulseCounter {
    pub fn new() -> Self {
        Self {
            pulses:            Arc::new(Mutex::new(0)),
            started:           Arc::new(Mutex::new(false)),
            interrupt_enabled: Arc::new(Mutex::new(false)),
        }
    }

    pub fn inject_pulses(&self, n: u32) {
        let next = *self.pulses.lock().unwrap() + n;
        *self.pulses.lock().unwrap() = next;
    }

    pub fn is_started(&self) -> bool           { *self.started.lock().unwrap() }
    pub fn is_interrupt_enabled(&self) -> bool { *self.interrupt_enabled.lock().unwrap() }
}

impl PulseCounter for HostPulseCounter {
    fn start(&mut self) -> anyhow::Result<()> {
        *self.started.lock().unwrap()           = true;
        *self.interrupt_enabled.lock().unwrap() = true;
        Ok(())
    }

    fn enable_interrupt(&mut self) -> anyhow::Result<()> {
        *self.interrupt_enabled.lock().unwrap() = true;
        Ok(())
    }

    fn total_pulses(&self) -> u32 { *self.pulses.lock().unwrap() }
    fn reset(&mut self)           { *self.pulses.lock().unwrap() = 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accumulates_injected_pulses() {
        let mut c = HostPulseCounter::new();
        c.start().unwrap();
        c.inject_pulses(5);
        c.inject_pulses(3);
        assert_eq!(c.total_pulses(), 8);
    }

    #[test]
    fn resets_to_zero() {
        let mut c = HostPulseCounter::new();
        c.inject_pulses(10);
        c.reset();
        assert_eq!(c.total_pulses(), 0);
    }
}
