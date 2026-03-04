use crate::platform::traits::{Clock, ConnectionGuard, DataSink, Delay, PulseCounter};
use anyhow::{Context, Result};

/// Core flow monitoring orchestrator.
///
/// This module contains pure business logic - no hardware dependencies.
/// Directly implements the main monitoring loop with timing, interrupt re-arming,
/// connection gating, and conditional state updates (only on successful publish).
pub struct Engine<P, C, S, G, D> {
    pub pulse_counter: P,
    pub clock: C,
    pub sink: S,
    pub guard: G,
    pub delay: D,
    pub last_sample_time: u64,
    pub last_pulse_count: u32,
}

impl<P, C, S, G, D> Engine<P, C, S, G, D>
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

    /// Perform one engine cycle.
    pub fn tick(&mut self) -> Result<()> {
        // Always re-arm interrupt for next edge capture
        self.pulse_counter
            .enable_interrupt()
            .context("Failed to enable interrupt")?;

        let now = self.clock.time_now_in_millis();
        let elapsed = self.clock.elapsed_ms(self.last_sample_time);

        // If less than 1 second has passed, yield and continue
        if elapsed < 1_000 {
            self.delay.delay_ms(10);
            return Ok(());
        }

        // Only proceed if both WiFi and MQTT are connected
        if !self.guard.is_ready() {
            self.delay.delay_ms(100);
            return Ok(());
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

        Ok(())
    }

    /// Run the engine loop with a custom per-tick hook.
    ///
    /// # Arguments
    /// - `hook`: Closure called each tick. Receives `(engine, tick_count)` and can
    ///   perform custom actions before or instead of the standard `tick()`.
    /// - `max_ticks`: `None` for infinite loop, `Some(n)` for finite n ticks.
    pub fn run_loop<F>(&mut self, mut hook: F, max_ticks: Option<usize>) -> Result<()>
    where
        F: FnMut(&mut Self, usize) -> Result<()>,
    {
        let mut tick_count = 0;
        loop {
            hook(self, tick_count)?;
            tick_count += 1;
            if let Some(max) = max_ticks {
                if tick_count >= max {
                    break;
                }
            }
        }
        Ok(())
    }
}
