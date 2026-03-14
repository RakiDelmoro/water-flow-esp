use crate::traits::{Delay, TimeSource};

/// Production time source using ESP IDF timer
pub struct EspTimeSource;

#[cfg(target_os = "espidf")]
impl TimeSource for EspTimeSource {
    fn now_millis(&self) -> u64 {
        unsafe { (esp_idf_svc::sys::esp_timer_get_time() / 1000) as u64 }
    }
}

/// Production delay wrapper using FreeRTOS
pub struct EspDelay;

#[cfg(target_os = "espidf")]
impl Delay for EspDelay {
    fn delay_ms(&self, ms: u32) {
        esp_idf_hal::delay::FreeRtos::delay_ms(ms);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "espidf")]
    fn test_esp_time_source_returns_value() {
        let ts = EspTimeSource;
        let t = ts.now_millis();
        assert!(t >= 0);
    }

    #[test]
    #[cfg(target_os = "espidf")]
    fn test_esp_delay_does_not_panic() {
        let delay = EspDelay;
        delay.delay_ms(1); // Very short delay, should not panic
    }
}
