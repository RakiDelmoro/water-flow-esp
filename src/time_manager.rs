use log::info;
use std::sync::Mutex;

/// Global offset between UTC epoch time and boot monotonic time
/// Protected by Mutex to allow updates for drift correction
static TIME_OFFSET_MS: Mutex<Option<u64>> = Mutex::new(None);

/// Monotonic time in milliseconds since boot
fn boot_time_ms() -> u64 {
    unsafe { (esp_idf_svc::sys::esp_timer_get_time() / 1000) as u64 }
}

/// Sync time using timestamp received from MQTT broker.
///
/// This should be called when a timestamp message is received from the broker.
/// The timestamp should be Unix epoch milliseconds (e.g., "1773283993307").
///
/// Returns the calculated offset.
pub fn sync_time_from_mqtt(timestamp_ms: u64) -> u64 {
    let boot_ms = boot_time_ms();
    let offset = timestamp_ms.saturating_sub(boot_ms);

    // Store or update the offset (allows drift correction)
    if let Ok(mut guard) = TIME_OFFSET_MS.lock() {
        let was_synced = guard.is_some();
        *guard = Some(offset);
        if was_synced {
            info!(
                "Time offset updated from MQTT: {} ms (new offset: {})",
                timestamp_ms, offset
            );
        } else {
            info!(
                "Time synced from MQTT: {} ms (offset: {})",
                timestamp_ms, offset
            );
        }
    }

    offset
}

/// Check if time has been synchronized (offset is set)
pub fn is_time_synced() -> bool {
    TIME_OFFSET_MS.lock().map(|g| g.is_some()).unwrap_or(false)
}

/// Convert a boot-relative timestamp to an absolute UTC timestamp
///
/// If time has not been synced yet, returns the boot_timestamp_ms as-is
/// (relative timestamps until sync is established).
pub fn boot_to_absolute_timestamp(boot_timestamp_ms: u64) -> u64 {
    let offset = TIME_OFFSET_MS.lock().ok().and_then(|g| *g).unwrap_or(0);
    boot_timestamp_ms + offset
}
