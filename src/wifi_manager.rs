//! WiFi Manager - Event-Driven with Fast-Fail
//!
//! Self-powered: no retry delays, fail fast, log and continue.

use crate::main_config::{WIFI_PASSWORD, WIFI_SSID};
use anyhow::Context;
use esp_idf_hal::modem::Modem;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{AuthMethod, ClientConfiguration, Configuration, EspWifi, ScanMethod};
use heapless::String;
use log::{debug, info, warn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc::Sender, Arc, Mutex};

use crate::AppEvent;

/// Setup WiFi driver
pub fn setup_wifi(modem: Modem) -> anyhow::Result<EspWifi<'static>> {
    let ssid: String<32> =
        String::try_from(WIFI_SSID).map_err(|_| anyhow::anyhow!("SSID too long"))?;
    let password: String<64> =
        String::try_from(WIFI_PASSWORD).map_err(|_| anyhow::anyhow!("Password too long"))?;

    let sysloop = EspSystemEventLoop::take().context("Event loop failed")?;
    let nvs = EspDefaultNvsPartition::take().context("NVS failed")?;

    let mut wifi = EspWifi::new(modem, sysloop, Some(nvs)).context("WiFi init failed")?;

    let config = ClientConfiguration {
        ssid,
        password,
        auth_method: AuthMethod::WPA2Personal,
        channel: Some(40),
        scan_method: ScanMethod::FastScan,
        ..Default::default()
    };

    wifi.set_configuration(&Configuration::Client(config))
        .context("WiFi config failed")?;

    wifi.start()?;
    info!(target: "wifi", "WiFi started");

    Ok(wifi)
}

/// Run WiFi loop - event-driven, fast-fail
pub fn run_wifi_loop(mut wifi: EspWifi<'static>, is_wifi_connected: Arc<AtomicBool>, event_tx: Arc<Mutex<Sender<AppEvent>>>) -> anyhow::Result<()> {
    // Try initial connection
    let _ = wifi.connect();

    loop {
        let connected = wifi.is_connected()? && wifi.is_up()?;
        let was_connected = is_wifi_connected.load(Ordering::Relaxed);

        if connected != was_connected {
            if connected {
                info!(target: "wifi", "Connected");
            } else {
                warn!(target: "wifi", "Disconnected");
            }

            is_wifi_connected.store(connected, Ordering::Relaxed);

            // Send event to main loop
            if let Ok(tx) = event_tx.lock() {
                let _ = tx.send(AppEvent::WifiState(connected));
            }
        }

        if !connected {
            // Try reconnect once - no delay
            match wifi.connect() {
                Ok(_) => debug!(target: "wifi", "Reconnected"),
                Err(e) => debug!(target: "wifi", "Reconnect failed: {:?}", e),
            }
            // Minimal yield - no busy wait
            std::thread::yield_now();
        }
    }
}
