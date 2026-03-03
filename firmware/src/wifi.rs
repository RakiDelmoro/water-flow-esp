use crate::traits::WifiAdapter;
use anyhow::Result;
use log::info;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[cfg(target_os = "espidf")]
use esp_idf_hal::modem::Modem;
#[cfg(target_os = "espidf")]
use esp_idf_svc::eventloop::EspSystemEventLoop;
#[cfg(target_os = "espidf")]
use esp_idf_svc::nvs::EspDefaultNvsPartition;
#[cfg(target_os = "espidf")]
use esp_idf_svc::wifi::EspWifi;
#[cfg(target_os = "espidf")]
use heapless::String;

/// Generic WiFi connection step function - testable on any platform
pub fn step_wifi<W>(wifi: &mut W, wifi_connected: &Arc<AtomicBool>) -> Result<()>
where W: WifiAdapter {
    let is_ready = wifi.is_connected()?;
    match is_ready {
        true => {
            if !wifi_connected.load(Ordering::Relaxed) {
                info!("WiFi connected!");
                wifi_connected.store(true, Ordering::Relaxed);
            }
        }
        false => {
            if wifi_connected.load(Ordering::Relaxed) {
                info!("WiFi disconnected!");
                wifi_connected.store(false, Ordering::Relaxed);
            }
            match wifi.connect() {
                Ok(_) => {
                    info!("WiFi reconnection initiated");
                }
                Err(e) => {
                    info!("WiFi reconnection failed: {:?}, retrying...", e);
                }
            }
        }
    }
    Ok(())
}

/// Run WiFi connection management loop
pub fn run_wifi_loop<W>(mut wifi: W, wifi_connected: Arc<AtomicBool>) -> Result<()>
where W: WifiAdapter {
    loop {
        step_wifi(&mut wifi, &wifi_connected)?;
    }
}

#[cfg(target_os = "espidf")]
/// Production WiFi adapter wrapping ESP-IDF WiFi stack
pub struct EspWifiAdapter {
    wifi: EspWifi<'static>,
}

#[cfg(target_os = "espidf")]
impl EspWifiAdapter {
    pub fn new(modem: Modem) -> Result<Self> {
        let ssid_as_heap_string = String::<32>::try_from(crate::config::WIFI_SSID).expect("SSID too long");
        let password_as_heap_string = String::<64>::try_from(crate::config::WIFI_PASSWORD).expect("Password too long");

        let sysloop = EspSystemEventLoop::take().expect("Failed to take event loop");
        let nvs = EspDefaultNvsPartition::take().expect("Failed to take NVS");

        let mut wifi = EspWifi::new(modem, sysloop.clone(), Some(nvs)).expect("Failed to initialize WiFi");

        let wifi_config = esp_idf_svc::wifi::ClientConfiguration {
            ssid: ssid_as_heap_string,
            password: password_as_heap_string,
            auth_method: esp_idf_svc::wifi::AuthMethod::WPA2Personal,
            channel: Some(40),
            scan_method: esp_idf_svc::wifi::ScanMethod::FastScan,
            ..Default::default()
        };

        wifi.set_configuration(&esp_idf_svc::wifi::Configuration::Client(wifi_config)).expect("Failed to set WiFi");
        wifi.start()?;

        Ok(Self { wifi })
    }
}

#[cfg(target_os = "espidf")]
impl WifiAdapter for EspWifiAdapter {
    fn is_connected(&self) -> Result<bool> {
        Ok(self.wifi.is_connected()? && self.wifi.is_up()?)
    }

    fn connect(&mut self) -> Result<()> {
        self.wifi.connect().map_err(Into::into)
    }
}

#[cfg(target_os = "espidf")]
/// Initialize WiFi and return adapter
pub fn setup_wifi(modem: Modem) -> Result<EspWifiAdapter> {
    EspWifiAdapter::new(modem)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::WifiAdapter;
    use anyhow::anyhow;
    use mockall::mock;
    use std::env::consts::ARCH;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    mock! {
        TestWifi {}
        impl WifiAdapter for TestWifi {
            fn is_connected(&self) -> Result<bool>;
            fn connect(&mut self) -> Result<()>;
        }
    }

    #[test]
    fn wifi_sets_connected_flag_when_connected() {
        let mut wifi = MockTestWifi::new();
        let connected_flag = Arc::new(AtomicBool::new(false));

        wifi.expect_is_connected().returning(|| Ok(true));

        step_wifi(&mut wifi, &connected_flag).unwrap();

        assert!(connected_flag.load(Ordering::Relaxed));
    }

    #[test]
    fn wifi_clears_flag_when_disconnected() {
        let mut wifi = MockTestWifi::new();
        let connected_flag = Arc::new(AtomicBool::new(true));

        wifi.expect_is_connected().returning(|| Ok(false));
        wifi.expect_connect().returning(|| Ok(()));

        step_wifi(&mut wifi, &connected_flag).unwrap();

        assert!(!connected_flag.load(Ordering::Relaxed));
    }

    #[test]
    fn wifi_calls_connect_when_disconnected() {
        let mut wifi = MockTestWifi::new();
        let connected_flag = Arc::new(AtomicBool::new(true));

        wifi.expect_is_connected().returning(|| Ok(false));
        wifi.expect_connect().returning(|| Ok(()));

        step_wifi(&mut wifi, &connected_flag).unwrap();
    }

    #[test]
    fn wifi_handles_connect_error() {
        let mut wifi = MockTestWifi::new();
        let connected_flag = Arc::new(AtomicBool::new(true));

        wifi.expect_is_connected().returning(|| Ok(false));
        wifi.expect_connect().returning(|| Err(anyhow!("connection failed")));

        step_wifi(&mut wifi, &connected_flag).unwrap();

        assert!(!connected_flag.load(Ordering::Relaxed));
    }

    #[test]
    fn wifi_reconnects_successfully() {
        let mut wifi = MockTestWifi::new();
        let connected_flag = Arc::new(AtomicBool::new(true));

        // First call: Disconnected -> should call connect()
        wifi.expect_is_connected().times(1).returning(|| Ok(false));
        wifi.expect_connect().times(1).returning(|| Ok(()));

        step_wifi(&mut wifi, &connected_flag).unwrap();
        // Flag cleared because disconnected
        assert!(!connected_flag.load(Ordering::Relaxed));

        // Second call: reconnected
        wifi.expect_is_connected().times(1).returning(|| Ok(true));

        step_wifi(&mut wifi, &connected_flag).unwrap();
        // Flag set again because reconnected
        assert!(connected_flag.load(Ordering::Relaxed));
    }
}
