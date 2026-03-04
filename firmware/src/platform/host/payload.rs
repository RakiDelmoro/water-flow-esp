use crate::platform::traits::PayloadBuilder;

pub struct HostPayloadBuilder;

impl PayloadBuilder for HostPayloadBuilder {
    /// CSV bytes — trivially assertable without a JSON parser.
    fn build(
        &self,
        pulse_delta: u32,
        time_delta_ms: u64,
        accumulative_pulse: u32,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(format!("{pulse_delta},{time_delta_ms},{accumulative_pulse}").into_bytes())
    }
}
