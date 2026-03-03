use crate::platform::traits::{Clock, PayloadSample, PayloadSampler, PulseCounter};

pub struct HardwarePayloadSampler<C: Clock, P: PulseCounter> {
    clock:        C,
    counter:      P,
    last_ms:      u64,
    last_pulses:  u32,
    accumulative: u32,
    interval_ms:  u64,
}

impl<C: Clock, P: PulseCounter> HardwarePayloadSampler<C, P> {
    pub fn new(clock: C, mut counter: P, interval_ms: u64) -> anyhow::Result<Self> {
        counter.start()?;
        let last_ms = clock.time_now_in_millis();
        Ok(Self { clock, counter, last_ms, last_pulses: 0, accumulative: 0, interval_ms })
    }
}

fn compute_delta(last: u32, current: u32, accumulative: u32) -> (u32, u32) {
    let delta = current.saturating_sub(last);
    (delta, accumulative + delta)
}

impl<C: Clock, P: PulseCounter> PayloadSampler for HardwarePayloadSampler<C, P> {
    fn poll(&mut self) -> Option<FlowSample> {
        let now     = self.clock.time_now_in_millis();
        let elapsed = now.saturating_sub(self.last_ms);

        (elapsed >= self.interval_ms).then(|| {
            let (delta, new_acc) =
                compute_delta(self.last_pulses, self.counter.total_pulses(), self.accumulative);

            self.last_ms      = now;
            self.last_pulses  = self.counter.total_pulses();
            self.accumulative = new_acc;
            let _ = self.counter.enable_interrupt();

            PayloadSample { pulse_delta: delta, time_delta_ms: elapsed, accumulative_pulse: new_acc }
        })
    }
}
