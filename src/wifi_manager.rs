use crate::main_config::{WIFI_PASSWORD, WIFI_SSID};
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::modem::Modem;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::ipv4::{
    ClientConfiguration as IpClientConfiguration, ClientSettings as IpClientSettings,
    Configuration as IpConfiguration, Mask, Subnet,
};
use esp_idf_svc::netif::{EspNetif, NetifConfiguration, NetifStack};
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{AuthMethod, ClientConfiguration, Configuration, EspWifi};
use heapless::String;
use log::info;
use std::net::Ipv4Addr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub fn setup_wifi(modem: Modem) -> anyhow::Result<EspWifi<'static>> {
    let ssid_as_heap_string: String<32> = String::try_from(WIFI_SSID).expect("SSID too long");
    let password_as_heap_string: String<64> =
        String::try_from(WIFI_PASSWORD).expect("Password too long");

    let sysloop = EspSystemEventLoop::take().expect("Failed to take event loop");
    let nvs = EspDefaultNvsPartition::take().expect("Failed to take NVS");

    // Static IP configuration
    let static_ip = Ipv4Addr::new(000, 000, 000, 000);
    let gateway = Ipv4Addr::new(000, 000, 000, 000);
    let netmask = Mask(24); // 255.255.255.0

    let netif_config = NetifConfiguration {
        ip_configuration: Some(IpConfiguration::Client(IpClientConfiguration::Fixed(
            IpClientSettings {
                ip: static_ip,
                subnet: Subnet {
                    gateway,
                    mask: netmask,
                },
                dns: Some(gateway), // Use gateway as DNS
                secondary_dns: None,
            },
        ))),
        ..NetifConfiguration::wifi_default_client()
    };

    let sta_netif = EspNetif::new_with_conf(&netif_config)?;
    let ap_netif = EspNetif::new(NetifStack::Ap)?;

    let mut wifi = EspWifi::wrap_all(
        esp_idf_svc::wifi::WifiDriver::new(modem, sysloop.clone(), Some(nvs))?,
        sta_netif,
        ap_netif,
    )?;

    let wifi_config = ClientConfiguration {
        ssid: ssid_as_heap_string,
        password: password_as_heap_string,
        auth_method: AuthMethod::WPA2Personal,
        ..Default::default()
    };

    wifi.set_configuration(&Configuration::Client(wifi_config)).expect("Failed to set WiFi configurations");
    wifi.start().expect("Failed to start WiFi");
    wifi.connect().expect("Failed to initiate WiFi connect");

    anyhow::Ok(wifi)
}

pub fn run_wifi_loop(mut wifi: EspWifi<'static>, wifi_connected: Arc<AtomicBool>) -> anyhow::Result<()> {
    let mut reconnect_attempts: u32 = 0;
    const MAX_BACKOFF_SECS: u64 = 30;

    loop {
        let is_ready = wifi.is_connected()? && wifi.is_up()?;
        match is_ready {
            true => {
                if !wifi_connected.load(Ordering::Relaxed) {
                    info!("WiFi connected!");
                    wifi_connected.store(true, Ordering::Relaxed);
                    reconnect_attempts = 0; // Reset counter on successful connection
                }
            }
            false => {
                if wifi_connected.load(Ordering::Relaxed) {
                    info!("WiFi disconnected!");
                    wifi_connected.store(false, Ordering::Relaxed);
                }

                // Only attempt reconnection if not already connecting
                match wifi.connect() {
                    Ok(_) => {
                        info!("WiFi reconnection initiated");
                        reconnect_attempts += 1;
                    }
                    Err(e) => {
                        // If already connecting, skip immediate retry
                        if e.to_string().contains("ESP_ERR_WIFI_CONN") {
                            // Already connecting, will retry after backoff
                        } else {
                            info!("WiFi reconnection error: {:?}", e);
                        }
                        reconnect_attempts += 1;
                    }
                }

                // Exponential backoff: 2^attempts seconds, capped at MAX_BACKOFF_SECS
                if reconnect_attempts > 0 {
                    let backoff_secs =
                        std::cmp::min(2_u64.pow(reconnect_attempts - 1), MAX_BACKOFF_SECS);
                    info!(
                        "Waiting {} seconds before next WiFi reconnect attempt...",
                        backoff_secs
                    );
                    FreeRtos::delay_ms((backoff_secs * 1000) as u32);
                }
            }
        }
    }
}
