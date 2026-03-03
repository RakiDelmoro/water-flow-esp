use crate::platform::traits::{Clock, ConnectionGuard, DataSink, Delay, PulseCounter};
use anyhow::{Context, Result};

/// Core flow monitoring orchestrator.
///
/// This module contains pure business logic - no hardware dependencies.
/// Directly implements the main monitoring loop with timing, interrupt re-arming,
/// connection gating, and conditional state updates (only on successful publish).
pub struct FlowMonitor<P, C, S, G, D> {
    pulse_counter: P,
    clock: C,
    sink: S,
    guard: G,
    delay: D,
    last_sample_time: u64,
    last_pulse_count: u32,
}

impl<P, C, S, G, D> FlowMonitor<P, C, S, G, D>
where
    P: PulseCounter,
    C: Clock,
    S: DataSink,
    G: ConnectionGuard,
    D: Delay,
{
    /// Create a new FlowMonitor with its dependencies.
    ///
    /// Calls `start()` on the pulse counter to attach the ISR and enable interrupts.
    pub fn new(
        mut pulse_counter: P,
        clock: C,
        sink: S,
        guard: G,
        delay: D,
    ) -> anyhow::Result<Self> {
        pulse_counter.start()?;
        let last_sample_time = clock.time_now_in_millis();
        let last_pulse_count = pulse_counter.total_pulses();
        Ok(Self {
            pulse_counter,
            clock,
            sink,
            guard,
            delay,
            last_sample_time,
            last_pulse_count,
        })
    }

    /// Start the monitoring loop.
    ///
    /// Loop logic (matches reference implementation):
    /// 1. Re-arm interrupt every iteration
    /// 2. Early exit if less than 1 second elapsed (delay 10ms)
    /// 3. Only publish when both WiFi and MQTT are ready (else delay 100ms)
    /// 4. On successful publish, update baselines
    /// 5. On error, log and keep baselines unchanged (retry same delta)
    pub fn start(&mut self) -> Result<()> {
        loop {
            // Always re-arm interrupt for next edge capture
            self.pulse_counter
                .enable_interrupt()
                .context("Failed to enable interrupt")?;

            let now = self.clock.time_now_in_millis();
            let elapsed = self.clock.elapsed_ms(self.last_sample_time);

            // If less than 1 second has passed, yield and continue
            if elapsed < 1_000 {
                self.delay.delay_ms(10);
                continue;
            }

            // Only proceed if both WiFi and MQTT are connected
            if !self.guard.is_ready() {
                self.delay.delay_ms(100);
                continue;
            }

            let current_pulses = self.pulse_counter.total_pulses();
            let time_delta = elapsed;
            let pulse_delta = current_pulses.saturating_sub(self.last_pulse_count);

            // Attempt to publish
            let sample = crate::platform::traits::PayloadSample {
                pulse_delta,
                time_delta_ms: time_delta,
                accumulative_pulse: self.last_pulse_count,
            };

            match self.sink.send(&sample) {
                Ok(()) => {
                    // Update baselines only on successful publish
                    self.last_pulse_count += pulse_delta;
                    self.last_sample_time = now;
                }
                Err(e) => {
                    log::error!("Failed to publish data: {e}");
                    // Baselines unchanged, will retry same delta next loop
                }
            }

            // Loop immediately without extra delay (early-exit will add ~10ms yield)
        }
    }
}
