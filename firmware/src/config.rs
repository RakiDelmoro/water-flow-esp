//! Application configuration loaded from environment variables.
//!
//! Centralizes all runtime configuration with sensible defaults.
//! Uses anyhow for error reporting.

use anyhow::{Context, Result};

/// Unified configuration for the firmware.
#[derive(Debug, Clone)]
pub struct Config {
    /// WiFi network SSID (required)
    pub wifi_ssid: String,
    /// WiFi password (required)
    pub wifi_pass: String,
    /// MQTT broker URL (required), e.g., `mqtt://broker.example.com:1883`
    pub mqtt_broker_url: String,
    /// MQTT client ID (required), unique identifier for this device
    pub mqtt_client_id: String,
    /// MQTT username (optional)
    pub mqtt_username: Option<String>,
    /// MQTT password (optional)
    pub mqtt_password: Option<String>,
    /// MQTT topic to publish to (default: "water/flow")
    pub mqtt_topic: String,
    /// Device identifier included in payloads (default: "esp32-flow")
    pub device_id: String,
    /// GPIO pin number for the flow sensor (default: 4)
    pub flow_sensor_pin: u8,
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// # Required variables
    /// - `WIFI_SSID`
    /// - `WIFI_PASS`
    /// - `MQTT_BROKER_URL`
    /// - `MQTT_CLIENT_ID`
    ///
    /// # Optional variables (with defaults)
    /// - `MQTT_TOPIC` -> "water/flow"
    /// - `DEVICE_ID` -> "esp32-flow"
    /// - `FLOW_SENSOR_PIN` -> 4
    /// - `MQTT_USERNAME` -> none
    /// - `MQTT_PASSWORD` -> none
    pub fn from_env() -> Result<Self> {
        let wifi_ssid = std::env::var("WIFI_SSID")
            .context("Missing required environment variable: WIFI_SSID")?;

        let wifi_pass = std::env::var("WIFI_PASS")
            .context("Missing required environment variable: WIFI_PASS")?;

        let mqtt_broker_url = std::env::var("MQTT_BROKER_URL")
            .context("Missing required environment variable: MQTT_BROKER_URL")?;

        let mqtt_client_id = std::env::var("MQTT_CLIENT_ID")
            .context("Missing required environment variable: MQTT_CLIENT_ID")?;

        let mqtt_username = std::env::var("MQTT_USERNAME").ok();
        let mqtt_password = std::env::var("MQTT_PASSWORD").ok();

        let mqtt_topic = std::env::var("MQTT_TOPIC").unwrap_or_else(|_| "water/flow".to_string());

        let device_id = std::env::var("DEVICE_ID").unwrap_or_else(|_| "esp32-flow".to_string());

        let flow_sensor_pin = std::env::var("FLOW_SENSOR_PIN")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(4);

        Ok(Config {
            wifi_ssid,
            wifi_pass,
            mqtt_broker_url,
            mqtt_client_id,
            mqtt_username,
            mqtt_password,
            mqtt_topic,
            device_id,
            flow_sensor_pin,
        })
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            wifi_ssid: "test-ssid".into(),
            wifi_pass: "test-pass".into(),
            mqtt_broker_url: "mqtt://localhost:1883".into(),
            mqtt_client_id: "test-client".into(),
            mqtt_username: None,
            mqtt_password: None,
            mqtt_topic: "water/flow".into(),
            device_id: "test-device".into(),
            flow_sensor_pin: 4,
        }
    }
}
