use crate::platform::traits::WifiManager;
use esp_idf_hal::modem::Modem;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    nvs::EspDefaultNvsPartition,
    wifi::{BlockingWifi, ClientConfiguration, Configuration, EspWifi},
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub struct Esp32WifiManager {
    wifi: BlockingWifi<EspWifi<'static>>,
    ssid: String,
    password: String,
}

impl WifiManager<Modem> for Esp32WifiManager {
    fn setup(modem: Modem, ssid: &str, password: &str) -> anyhow::Result<Self> {
        let sysloop = EspSystemEventLoop::take()?;
        let nvs = EspDefaultNvsPartition::take()?;
        EspWifi::new(modem, sysloop.clone(), Some(nvs))
            .map_err(anyhow::Error::from)
            .and_then(|esp_wifi| BlockingWifi::wrap(esp_wifi, sysloop).map_err(anyhow::Error::from))
            .map(|wifi| Self {
                wifi,
                ssid: ssid.to_string(),
                password: password.to_string(),
            })
    }

    fn run_loop(mut self, connected: Arc<AtomicBool>) -> anyhow::Result<()> {
        self.wifi
            .set_configuration(&Configuration::Client(ClientConfiguration {
                ssid: self.ssid.as_bytes().try_into().unwrap(),
                password: self.password.as_bytes().try_into().unwrap(),
                ..Default::default()
            }))?;
        self.wifi.start()?;
        std::iter::repeat(()).try_for_each(|_| connect_and_monitor(&mut self.wifi, &connected))
    }

    fn is_connected(&self) -> bool {
        self.wifi.is_connected().unwrap_or(false)
    }
}

fn connect_and_monitor(
    wifi: &mut BlockingWifi<EspWifi<'static>>,
    connected: &Arc<AtomicBool>,
) -> anyhow::Result<()> {
    wifi.connect()
        .and_then(|_| wifi.wait_netif_up())
        .map(|_| {
            connected.store(true, Ordering::Relaxed);
            log::info!("WiFi connected.");
        })
        .unwrap_or_else(|e| log::error!("WiFi error: {e}, retrying..."));

    while wifi.is_connected().unwrap_or(false) {
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
    connected.store(false, Ordering::Relaxed);
    log::warn!("WiFi dropped, reconnecting...");
    Ok(())
}
