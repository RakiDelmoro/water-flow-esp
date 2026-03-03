use crate::platform::traits::WifiManager;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub struct HostWifiManager {
    connected: bool,
}

impl HostWifiManager {
    pub const fn new(connected: bool) -> Self {
        Self { connected }
    }
}

impl WifiManager<()> for HostWifiManager {
    fn setup(_modem: ()) -> anyhow::Result<Self> {
        Ok(Self::new(true))
    }

    fn run_loop(self, connected: Arc<AtomicBool>) -> anyhow::Result<()> {
        connected.store(self.connected, Ordering::Relaxed);
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }
}
