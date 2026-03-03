use std::collections::VecDeque;
use crate::platform::traits::{PayloadSample, PayloadSampler};

pub struct HostFlowSampler(VecDeque<PayloadSample>);

impl HostFlowSampler {
    pub fn new() -> Self { Self(VecDeque::new()) }

    pub fn from_samples(samples: impl IntoIterator<Item = PayloadSample>) -> Self {
        Self(samples.into_iter().collect())
    }

    pub fn enqueue(&mut self, sample: PayloadSample) { self.0.push_back(sample) }
    pub fn remaining(&self) -> usize              { self.0.len() }
}

impl PayloadSampler for HostFlowSampler {
    fn poll(&mut self) -> Option<PayloadSample> { self.0.pop_front() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drains_in_fifo_order() {
        let mut s = HostFlowSampler::from_samples(
            (0..3u32).map(|i| PayloadSample { pulse_delta: i, time_delta_ms: i as u64 * 1_000, accumulative_pulse: i })
        );
        assert_eq!(s.poll().unwrap().pulse_delta, 0);
        assert_eq!(s.poll().unwrap().pulse_delta, 1);
        assert_eq!(s.poll().unwrap().pulse_delta, 2);
        assert!(s.poll().is_none());
    }
}
