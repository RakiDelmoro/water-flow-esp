use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use esp_idf_hal::modem::Modem;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    nvs::EspDefaultNvsPartition,
    wifi::{BlockingWifi, ClientConfiguration, Configuration, EspWifi},
};
use crate::platform::traits::WifiManager;

pub struct Esp32WifiManager {
    wifi: BlockingWifi<EspWifi<'static>>,
}

impl WifiManager<Modem> for Esp32WifiManager {
    fn setup(modem: Modem) -> anyhow::Result<Self> {
        let sysloop = EspSystemEventLoop::take()?;
        let nvs     = EspDefaultNvsPartition::take()?;
        EspWifi::new(modem, sysloop.clone(), Some(nvs))
            .map_err(anyhow::Error::from)
            .and_then(|esp_wifi| BlockingWifi::wrap(esp_wifi, sysloop).map_err(anyhow::Error::from))
            .map(|wifi| Self { wifi })
    }

    fn run_loop(mut self, connected: Arc<AtomicBool>) -> anyhow::Result<()> {
        self.wifi.set_configuration(&Configuration::Client(ClientConfiguration {
            ssid:     env!("WIFI_SSID").try_into().unwrap(),
            password: env!("WIFI_PASS").try_into().unwrap(),
            ..Default::default()
        }))?;
        self.wifi.start()?;
        std::iter::repeat(())
            .try_for_each(|_| connect_and_monitor(&mut self.wifi, &connected))
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
