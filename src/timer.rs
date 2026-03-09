//! Hardware timer for periodic sampling
//!
//! Uses ESP-IDF esp_timer for precise 1-second intervals.
//! Sends events via channel - no busy-waiting.

use esp_idf_svc::sys::{
    esp_timer_create, esp_timer_create_args_t, esp_timer_delete,
    esp_timer_dispatch_t_ESP_TIMER_TASK, esp_timer_get_time, esp_timer_handle_t,
    esp_timer_start_periodic, esp_timer_stop, ESP_OK,
};
use log::info;
use std::ffi::c_void;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::sync::Mutex;

use crate::AppEvent;

// Timer period in microseconds
const TIMER_PERIOD_US: u64 = 1_000_000; // 1 second

/// Timer callback - sends event to main loop
extern "C" fn timer_callback(arg: *mut c_void) {
    unsafe {
        if !arg.is_null() {
            let sender = &*(arg as *const Arc<Mutex<Sender<AppEvent>>>);
            if let Ok(tx) = sender.lock() {
                let _ = tx.send(AppEvent::TimerTick);
            }
        }
    }
}

/// Sample timer struct
pub struct SampleTimer {
    handle: esp_timer_handle_t,
}

impl SampleTimer {
    /// Create and start timer
    pub fn new(event_tx: Arc<Mutex<Sender<AppEvent>>>) -> anyhow::Result<Self> {
        // Convert Arc to raw pointer for C callback
        let sender_ptr = Arc::into_raw(event_tx) as *mut c_void;

        let timer_config = esp_timer_create_args_t {
            callback: Some(timer_callback),
            arg: sender_ptr,
            dispatch_method: esp_timer_dispatch_t_ESP_TIMER_TASK,
            name: b"flow_sample\0".as_ptr() as *const u8,
            skip_unhandled_events: false,
        };

        let mut handle: esp_timer_handle_t = std::ptr::null_mut();

        let ret = unsafe { esp_timer_create(&timer_config, &mut handle) };
        if ret != ESP_OK {
            anyhow::bail!("Timer create failed: {}", ret);
        }

        let ret = unsafe { esp_timer_start_periodic(handle, TIMER_PERIOD_US) };
        if ret != ESP_OK {
            unsafe { esp_timer_delete(handle) };
            anyhow::bail!("Timer start failed: {}", ret);
        }

        info!(target: "timer", "Started: {}us period", TIMER_PERIOD_US);
        Ok(Self { handle })
    }
}

impl Drop for SampleTimer {
    fn drop(&mut self) {
        unsafe {
            esp_timer_stop(self.handle);
            esp_timer_delete(self.handle);
        }
    }
}

/// Get current time in milliseconds
pub fn now_ms() -> u64 {
    unsafe { (esp_timer_get_time() / 1000) as u64 }
}
