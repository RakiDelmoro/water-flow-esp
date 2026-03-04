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
    fn setup(_modem: (), _ssid: &str, _password: &str) -> anyhow::Result<Self> {
        Ok(Self::new(true))
    }

    fn run_loop(
        self,
        connected: Arc<AtomicBool>,
        shutdown: Option<Arc<AtomicBool>>,
    ) -> anyhow::Result<()> {
        connected.store(self.connected, Ordering::Relaxed);
        loop {
            if let Some(s) = &shutdown {
                if s.load(Ordering::Relaxed) {
                    break Ok(());
                }
            }
            // Sleep in small increments to remain responsive to shutdown
            for _ in 0..50 {
                // 5 seconds total (but now we'll make each 10ms for faster sim)
                if let Some(s) = &shutdown {
                    if s.load(Ordering::Relaxed) {
                        return Ok(());
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(10)); // fast sleep
            }
        }
    }

    fn is_connected(&self) -> bool {
        self.connected
    }
}
