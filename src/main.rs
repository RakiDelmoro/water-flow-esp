//! Water Flow ESP32 Application
//! Production entry point with clean initialization.

#[cfg(feature = "esp-idf")]
use esp_idf_sys::entry;

#[cfg(feature = "esp-idf")]
use esp_idf_svc::log::EspLogger;

#[cfg(feature = "esp-idf")]
use log::info;

#[cfg(feature = "esp-idf")]
use water_flow_esp::config::AppConfig;

#[cfg(feature = "esp-idf")]
use water_flow_esp::engine::WaterFlowApp;

#[cfg(feature = "esp-idf")]
use water_flow_esp::setup::{setup_mqtt, setup_pulse_source, setup_wifi};

#[cfg(feature = "esp-idf")]
#[entry]
fn main() -> ! {
    // Initialize logger
    EspLogger::default().init().unwrap();
    info!("Starting Water Flow ESP32 application");

    // Take singleton resources (only once for entire program)
    let mut peripherals = esp_idf_svc::hal::peripherals::Peripherals::take().unwrap();
    let sys_loop = esp_idf_svc::eventloop::EspSystemEventLoop::take().unwrap();
    let nvs = esp_idf_svc::nvs::EspDefaultNvsPartition::take().unwrap();

    // Load configuration from environment
    let config = AppConfig::load_config();
    info!("Configuration loaded: interval={}s", config.interval_secs);

    // Setup peripherals and services (automatically selects real or mock based on features)
    let pulse_source = setup_pulse_source(&mut peripherals).unwrap();
    let wifi = setup_wifi(peripherals, sys_loop, nvs).unwrap();
    let mqtt = setup_mqtt();

    info!("All services initialized, starting main loop");

    // Build and run the application
    let mut app = WaterFlowApp::new(pulse_source, wifi, mqtt, config);
    app.run()
}

// Fallback for when esp-idf is not enabled (should not happen in production builds)
#[cfg(not(feature = "esp-idf"))]
fn main() {
    panic!("This application requires the 'esp-idf' feature. Build with: cargo build --features esp-idf");
}
