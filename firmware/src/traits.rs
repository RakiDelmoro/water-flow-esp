/// Represents a flow measurement with pulse count and timestamp
#[derive(Debug, Clone, PartialEq)]
pub struct Measurement {
    pub pulses: u32,
    pub timestamp: u64,
}

/// Trait for getting current time in milliseconds
pub trait TimeSource {
    fn now_millis(&self) -> u64;
}

/// Trait for flow pulse counting with atomic swap operation
pub trait FlowCounter {
    /// Atomically read and optionally reset the counter
    fn swap(&self, reset: bool) -> u32;
}

/// Trait for WiFi connection management
pub trait WifiAdapter {
    /// Check if WiFi is connected and up
    fn is_connected(&self) -> anyhow::Result<bool>;
    /// Attempt to connect to WiFi
    fn connect(&mut self) -> anyhow::Result<()>;
}

/// Trait for MQTT publishing
pub trait MqttPublisher {
    fn publish(&mut self, topic: &str, data: &[u8], qos: i8, retain: bool) -> anyhow::Result<()>;
}

/// Trait for delaying execution
pub trait Delay {
    fn delay_ms(&self, ms: u32);
}
