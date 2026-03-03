use std::sync::{Arc, Mutex};

pub trait Clock {
    // Returns the current time in milliseconds since boot.
    fn time_now_in_millis(&self) -> u64;

    // Returns the elapsed milliseconds since_ms
    fn elapsed_ms(&self, since_ms: u64) -> u64 {
        self.time_now_in_millis().saturating_sub(since_ms)
    }
}

pub trait PulseCounter {
    /// Attach an interrupt handler and begin counting pulses.
    fn start(&mut self) -> anyhow::Result<()>;

    /// Re-arm the interrupt so the next edge is captured.
    /// Maps to `flow_pin.enable_interrupt()`.
    fn enable_interrupt(&mut self) -> anyhow::Result<()>;

    /// Return the current raw pulse count (monotonically increasing).
    fn total_pulses(&self) -> u32;

    /// Reset the counter back to zero.
    fn reset(&mut self);
}

pub trait WifiManager<M> {
    /// Perform one-time hardware initialisation and return a handle.
    fn setup(modem: M) -> anyhow::Result<Self>
    where Self: Sized;

    /// Blocking loop: connect, monitor, reconnect on drop.
    /// Writes `true` into `connected` when the link is up.
    fn run_loop(self, connected: Arc<std::sync::atomic::AtomicBool>) -> anyhow::Result<()>;

    /// Synchronous snapshot: is the interface currently associated?
    fn is_connected(&self) -> bool;
}

pub trait MqttManager {
    type Client: MqttPublisher;

    /// Blocking loop: wait for WiFi, connect to broker, maintain session.
    /// Populates `client_slot` once a session is established;
    /// clears it on disconnect.
    fn run_loop(wifi_ready: Arc<std::sync::atomic::AtomicBool>, mqtt_ready: Arc<std::sync::atomic::AtomicBool>, client_slot: Arc<Mutex<Option<Self::Client>>>) -> anyhow::Result<()>;
}

pub trait MqttPublisher {
    /// Publish `payload` bytes to `topic`.
    /// `retain` controls the broker's retain flag.
    fn publish(&mut self, topic: &str, retain: bool, payload: &[u8]) -> anyhow::Result<()>;
}

pub trait PayloadBuilder {
    fn build(&self, pulse_delta: u32, time_delta_ms: u64, accumulative_pulse: u32) -> anyhow::Result<Vec<u8>>;
}


pub struct PayloadSample {
    pub pulse_delta: u32,
    pub time_delta_ms: u64,
    pub accumulative_pulse: u32
}

pub trait PayloadSampler {
    fn poll(&mut self) -> Option<PayloadSample>;
}

pub trait ConnectionGuard {
    // Returns 'true' only when both WiFi and MQTT are ready.
    fn is_ready(&self) -> bool;
}

// Combines MqttPublisher + PayloadBuilder into a single "send" step.
// main loop calls this instead of interacting with the client directly.
pub trait DataSink {
    fn send(&mut self, sample: &PayloadSample) -> anyhow::Result<()>;
}

pub trait Delay {
    // Yield the CPU for at least milliseconds.
    fn delay_ms(&self, ms: u32);
}
