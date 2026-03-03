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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(d: u32) -> PayloadSample {
        PayloadSample {
            pulse_delta: d,
            time_delta_ms: 1_000,
            accumulative_pulse: d,
        }
    }

    #[test]
    fn records_samples_and_folds_totals() {
        let mut sink = HostDataSink::new();
        sink.send(&sample(2)).unwrap();
        sink.send(&sample(3)).unwrap();
        assert_eq!(sink.sent_count(), 2);
        assert_eq!(sink.total_pulses_sent(), 5);
        assert_eq!(sink.last_sent().unwrap().pulse_delta, 3);
    }

    #[test]
    fn fail_next_recovers_on_next_call() {
        let mut sink = HostDataSink {
            sent: vec![],
            fail_next: true,
        };
        assert!(sink.send(&sample(1)).is_err());
        assert!(sink.send(&sample(1)).is_ok());
        assert_eq!(sink.sent_count(), 1);
    }
}
