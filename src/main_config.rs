//! Main configuration constants

// WiFi Configurations
pub const WIFI_SSID: &str = "";
pub const WIFI_PASSWORD: &str = "";

// MQTT Configurations
pub const MQTT_TOPIC: &str = "esp/water-flow";
pub const MQTT_USERNAME: &str = "";
pub const MQTT_PASSWORD: &str = "";
pub const MQTT_URL: &str = "";

// Static IP Configuration (used by connection_manager.rs)
// Use [0,0,0,0] for DHCP (default behavior preserved)
pub const STATIC_IP: [u8; 4] = [0, 0, 0, 0];
pub const GATEWAY: [u8; 4] = [0, 0, 0, 0];
pub const NETMASK: u8 = 24; // CIDR prefix length (e.g., 24 for 255.255.255.0)
