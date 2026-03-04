use crate::platform::traits::{PayloadSample, PayloadSampler};
use std::collections::VecDeque;

pub struct HostFlowSampler(VecDeque<PayloadSample>);

impl HostFlowSampler {
    pub fn new() -> Self {
        Self(VecDeque::new())
    }

    pub fn from_samples(samples: impl IntoIterator<Item = PayloadSample>) -> Self {
        Self(samples.into_iter().collect())
    }

    pub fn enqueue(&mut self, sample: PayloadSample) {
        self.0.push_back(sample)
    }
    pub fn remaining(&self) -> usize {
        self.0.len()
    }
}

impl PayloadSampler for HostFlowSampler {
    fn poll(&mut self) -> Option<PayloadSample> {
        self.0.pop_front()
    }
}
