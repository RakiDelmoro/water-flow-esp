use crate::platform::traits::{DataSink, PayloadSample};

pub struct HostDataSink {
    pub sent: Vec<PayloadSample>,
    pub fail_next: bool,
}

impl HostDataSink {
    pub fn new() -> Self {
        Self {
            sent: Vec::new(),
            fail_next: false,
        }
    }

    pub fn sent_count(&self) -> usize {
        self.sent.len()
    }
    pub fn last_sent(&self) -> Option<&PayloadSample> {
        self.sent.last()
    }

    pub fn total_pulses_sent(&self) -> u32 {
        self.sent.iter().fold(0, |acc, s| acc + s.pulse_delta)
    }
}

impl DataSink for HostDataSink {
    fn send(&mut self, sample: &PayloadSample) -> anyhow::Result<()> {
        match std::mem::replace(&mut self.fail_next, false) {
            true => anyhow::bail!("injected DataSink send failure"),
            false => {
                self.sent.push(PayloadSample { ..*sample });
                Ok(())
            }
        }
    }
}
