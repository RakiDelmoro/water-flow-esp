use crate::platform::traits::Clock;

pub struct Esp32Clock;

impl Clock for Esp32Clock {
    fn time_now_in_millis(&self) -> u64 {
        unsafe { esp_idf_svc::sys::esp_timer_get_time() as u64 / 1_000 }
    }
}
